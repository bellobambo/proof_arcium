# Proof Arcium

Proof Arcium is a Solana devnet program for private exam assessment. It lets tutors create courses and exams, lets students enroll and request exam access, and grades submitted answers without exposing the tutor's answer.

## Program ID

Solana devnet program id:

```text
Ch5KUtPipgBTnjCVX1du7keV7pd6cdxJDLovRErFuSh
```

## What The Program Is For

This program is built for an education or assessment workflow where exam data should be verifiable on-chain, but sensitive grading data should stay private. Tutors can register, create courses, upload encrypted exam content, and store an encrypted answer key. Students can register, enroll in courses, request access to an exam, and submit answers.

After a student submits an exam, the program records the exam session and later stores the final score and correctness result once the private computation is complete.

## How I Used Arcium

I used Arcium to add confidential computation to the grading step. The answer key is stored as encrypted data, and the Arcium encrypted instruction compares the student's answers against that encrypted answer key inside Arcium MPC.

The confidential circuit is in:

```text
encrypted-ixs/src/lib.rs
```

It reveals only:

- the student's score
- a correctness mask showing which answers were correct

This means the program can publish useful grading results on Solana while keeping the answer key private.

## Main Flow

1. A user registers as a tutor or student.
2. A tutor creates a course.
3. A student enrolls in the course.
4. A tutor creates an exam with encrypted content and an encrypted answer key.
5. A student requests access and submits answers.
6. Arcium grades the answers privately.
7. The callback writes the final score to the student's exam session.

## Build And Test

Build the Arcium program:

```bash
NO_DNA=1 arcium build
```

Run Rust tests:

```bash
NO_DNA=1 cargo test -p proof_arcium
```
