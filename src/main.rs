use std::{cell::Cell, env, fs::File, path::Path, path::PathBuf, process};

use bincode::deserialize_from;
use proofman_util::VadcopFinalProof;
use proofman_verifier::{verify_vadcop_final_bytes, verify_vadcop_final_compressed_bytes};
use serde::{Deserialize, Serialize};

const ZISK_PUBLICS: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ZiskProof {
    Null(),
    VadcopFinal(Vec<u8>),
    VadcopFinalCompressed(Vec<u8>),
    Plonk(Vec<u8>),
    Fflonk(Vec<u8>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZiskProgramVK {
    pub vk: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZiskPublics {
    data: Vec<u8>,
    ptr: Cell<usize>,
}

impl ZiskPublics {
    pub fn public_bytes(&self) -> Vec<u8> {
        let mut bytes = [0u8; ZISK_PUBLICS * 8];
        for i in 0..ZISK_PUBLICS {
            let start = i * 4;
            let val32 = u32::from_le_bytes([
                self.data[start],
                self.data[start + 1],
                self.data[start + 2],
                self.data[start + 3],
            ]);
            let val64 = val32 as u64;
            bytes[i * 8..(i + 1) * 8].copy_from_slice(&val64.to_le_bytes());
        }
        bytes.to_vec()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZiskProofWithPublicValues {
    pub proof: ZiskProof,
    pub publics: ZiskPublics,
    pub program_vk: ZiskProgramVK,
}

impl ZiskProofWithPublicValues {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let file = File::open(path.as_ref()).map_err(|e| {
            std::io::Error::new(
                e.kind(),
                format!(
                    "Failed to open file for loading proof: {}: {}",
                    path.as_ref().display(),
                    e
                ),
            )
        })?;
        let proof_with_publics: ZiskProofWithPublicValues = deserialize_from(file)?;
        Ok(proof_with_publics)
    }

    pub fn get_vadcop_final_proof(&self) -> Result<VadcopFinalProof, String> {
        match &self.proof {
            ZiskProof::VadcopFinal(proof_bytes) | ZiskProof::VadcopFinalCompressed(proof_bytes) => {
                let compressed = matches!(self.proof, ZiskProof::VadcopFinalCompressed(_));
                let mut pubs = self.program_vk.vk.clone();
                pubs.extend(self.publics.public_bytes());
                Ok(VadcopFinalProof::new(proof_bytes.clone(), pubs, compressed))
            }
            _ => Err("Proof is not a Vadcop final proof".to_string()),
        }
    }
}

fn main() {
    let mut args = env::args().skip(1);
    let proof_path = match args.next() {
        Some(path) => PathBuf::from(path),
        None => {
            eprintln!(
                "usage: cargo run -- <wrapped_proof.bin> <vadcop_final.verkey.bin>"
            );
            process::exit(2);
        }
    };
    let vk_path = match args.next() {
        Some(path) => PathBuf::from(path),
        None => {
            eprintln!(
                "usage: cargo run -- <wrapped_proof.bin> <vadcop_final.verkey.bin>"
            );
            process::exit(2);
        }
    };

    let wrapped = match ZiskProofWithPublicValues::load(&proof_path) {
        Ok(v) => v,
        Err(err) => {
            eprintln!("failed to load wrapped proof {}: {}", proof_path.display(), err);
            process::exit(1);
        }
    };
    let vadcop = match wrapped.get_vadcop_final_proof() {
        Ok(v) => v,
        Err(err) => {
            eprintln!("failed to convert wrapped proof {}: {}", proof_path.display(), err);
            process::exit(1);
        }
    };
    let raw = vadcop.proof_with_publics();
    let vk = match std::fs::read(&vk_path) {
        Ok(v) => v,
        Err(err) => {
            eprintln!("failed to read vk {}: {}", vk_path.display(), err);
            process::exit(1);
        }
    };

    let ok = if vadcop.compressed {
        verify_vadcop_final_compressed_bytes(&raw, &vk)
    } else {
        verify_vadcop_final_bytes(&raw, &vk)
    };

    println!("proof={}", proof_path.display());
    println!("vk={}", vk_path.display());
    println!("compressed={}", vadcop.compressed);
    println!("verified={ok}");

    if !ok {
        process::exit(1);
    }
}
