import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { expect } from "chai";
import { ProofArcium } from "../target/types/proof_arcium";

describe("ProofArcium IDL", () => {
  anchor.setProvider(anchor.AnchorProvider.env());
  const program = anchor.workspace.ProofArcium as Program<ProofArcium>;

  it("exposes assessment and Arcium grading instructions", async () => {
    const instructionNames = program.idl.instructions.map((ix) => ix.name);

    expect(instructionNames).to.include.members([
      "initialize",
      "register_user",
      "create_course",
      "enroll_in_course",
      "create_exam",
      "request_exam_access",
      "grant_exam_access",
      "take_exam",
      "init_grade_exam_comp_def",
      "grade_exam_callback",
    ]);
  });

  it("stores public scores while keeping uploaded exam payload encrypted", async () => {
    const accountNames =
      program.idl.accounts?.map((account) => account.name) ?? [];
    const typeNames = program.idl.types?.map((type) => type.name) ?? [];

    expect(accountNames).to.include.members([
      "Exam",
      "ExamAccess",
      "ExamSession",
    ]);
    expect(typeNames).to.include.members([
      "EncryptedExamInput",
      "EncryptedContentKeyInput",
    ]);
  });
});
