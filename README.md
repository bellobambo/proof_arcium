# Proof Arcium

This is the Arcium-backed version of the proof assessment program.

The public Anchor program keeps the original course, enrollment, exam-session, score, and assessment verification flow. Tutor-uploaded exam content is the private surface:

- `Exam.content_ciphertexts` stores encrypted question/options payload chunks.
- `Exam.answer_key_ciphertexts` stores the MXE-encrypted answer key.
- `ExamAccess` records enrolled-student exam access and can store a per-student encrypted content key.
- `take_exam` stores the student's submitted answers and queues the Arcium `grade_exam` computation.
- `grade_exam_callback` writes the revealed score and correctness mask to `ExamSession`, so student scores remain publicly viewable and verifiable.

The confidential circuit lives in `encrypted-ixs/src/lib.rs`. It compares public student answers against the encrypted answer key inside Arcium MPC and reveals only:

- `score: u16`
- `correctness_mask: u32`

Student exam access flow:

1. Student enrolls in a course.
2. Student calls `request_exam_access` with their exam-content encryption public key. The program verifies enrollment and immediately marks access as granted.
3. Student frontend fetches `Exam.content_ciphertexts` plus whatever key-delivery material your app uses, then decrypts the questions/options locally.
4. Student submits answers with `take_exam`; the Arcium callback writes public scores.

The answer key is never revealed to the student. The exam content can be read only by students who are enrolled, have an `ExamAccess` account, and can decrypt the app-provided exam content key.

Build with:

```bash
NO_DNA=1 arcium build
```

Rust tests:

```bash
NO_DNA=1 cargo test -p proof_arcium
```
