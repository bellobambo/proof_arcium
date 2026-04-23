# Proof Arcium

This is the Arcium-backed version of the proof assessment program.

The public Anchor program keeps the original course, enrollment, exam-session, score, and assessment verification flow. Tutor-uploaded exam content is the private surface:

- `Exam.content_ciphertexts` stores encrypted question/options payload chunks.
- `Exam.answer_key_ciphertexts` stores the MXE-encrypted answer key.
- `ExamAccess` stores a per-student encrypted content key for enrolled students.
- `take_exam` stores the student's submitted answers and queues the Arcium `grade_exam` computation.
- `grade_exam_callback` writes the revealed score and correctness mask to `ExamSession`, so student scores remain publicly viewable and verifiable.

The confidential circuit lives in `encrypted-ixs/src/lib.rs`. It compares public student answers against the encrypted answer key inside Arcium MPC and reveals only:

- `score: u16`
- `correctness_mask: u32`

Student exam access flow:

1. Student enrolls in a course.
2. Student calls `request_exam_access` with their exam-content encryption public key.
3. Tutor or backend verifies the request account and calls `grant_exam_access` with the exam-content key encrypted to that student.
4. Student frontend fetches `Exam.content_ciphertexts` plus their `ExamAccess.encrypted_content_key`, decrypts the content key locally, then decrypts the questions/options.
5. Student submits answers with `take_exam`; the Arcium callback writes public scores.

The answer key is never revealed to the student. The exam content can be read only by students who have a granted `ExamAccess` account and can decrypt the per-student content key.

Build with:

```bash
NO_DNA=1 arcium build
```

Rust tests:

```bash
NO_DNA=1 cargo test -p proof_arcium
```
