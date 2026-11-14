# Bundled dictionary

`en_US.aff` and `en_US.dic` are the Hunspell English (United States) dictionary, built from SCOWL (Spell Checker Oriented Word Lists) by Kevin Atkinson. The files are copied unchanged from the Debian `hunspell-en-us` package.

They are **not** under the MIT OR Apache-2.0 license of the rest of this repository. Their copyright and permission notices are in [`en_US.LICENSE.txt`](en_US.LICENSE.txt) and must stay with the files, including in packaged builds.

The dictionary is compiled into the program, so spellcheck works with no network and no system dictionary.
