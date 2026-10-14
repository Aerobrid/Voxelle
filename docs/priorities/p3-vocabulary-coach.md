# P3 - Personal vocabulary and speech coach
Optional, local, never running continuously by default, and separate from the dictation path (no added latency).

## 3.1 Language profile
Built only from transcripts the user chose to retain. Track: frequent words/phrases, vague adjectives/adverbs, fillers and habits, confidently used vocabulary, known words, saved/learned words, preferred register, grammar patterns, goals. Use plain code for counts and statistics, not an LLM. Treat ASR errors as uncertain evidence; let users correct or exclude transcripts. Never infer vocabulary level from frequency alone.

## 3.2 Recommendation sources
A. Local lexical resources (WordNet or compatible): definitions, synonyms, relations, parts of speech. Optional dictionary data only with suitable redistribution rights.
B. Word-frequency data (licensed): commonness/rarity, never the only criterion.
C. Local LLM: find vague wording and propose context-fit candidates, explaining how candidates differ rather than treating them as interchangeable.
D. Personal history: suggested / saved / known / learned / practiced / dismissed; manual mark known or unfamiliar.

## 3.3 Pipeline
1 select eligible saved transcript or phrase; 2 extract sentences and candidate expressions; 3 flag vague/repetitive wording; 4 ask the local LLM for alternatives (advanced/literary if requested); 5 validate against the lexical database; 6 check part of speech, definition, register, commonness; 7 compare with user history; 8 rank by fit, precision, usefulness, novelty, naturalness; 9 show a short ranked list; 10 save / dismiss / mark known / practice.
Never replace the user's wording automatically. Each suggestion shows: word, pronunciation if available, part of speech, definition, register and usage notes, example sentence, original phrase, why it fits, differences from similar candidates. Teach appropriate usage; do not push needlessly complex speech.

## 3.4 UI
Discover Words action, daily/weekly picks, saved words, search/filter, examples, practice prompts, optional spaced repetition, difficulty and register controls (everyday precision, academic, literary, technical, historical, advanced). User can pause analysis, delete the profile, or disable transcript-based learning.

## 3.5 Speech feedback reports
Repeated phrases, fillers, vague wording, sentence-length variation, grammar patterns, vocabulary diversity (explain the formula), goal progress. Do not present these as authoritative measures of intelligence or fluency.

Definition of done: ranking and known-word exclusion are unit-tested; all data stays local; profile deletion is tested.
