# VBA compiled-code opcode reference

Companion to `pcode-format.md`. Describes the instruction words that fill the per-line code regions of a module's compiled cache.

Conventions used throughout:

* All multi-byte quantities are little-endian on PC projects (see the format document for the Mac big-endian case).
* **W** = one 16-bit operand word. **D** = one 32-bit operand (two words). **V** = variable-length trailer (see 1.3).
* "name" operand = a W that is an identifier reference (format document, section 5).
* "obj" operand = a W that references an entry of the module's object table (format document, section 6.1): the entry index times the pointer size (4 on x86, 8 on x64); the type/class name is the identifier reference found 6 bytes into the 10-byte entry (byte offset index times 10).
* "rec" operand = a D that is a byte offset into the module's indirect table (format document, section 6.2).
* Stack effects and semantics in the tables are the implementer-facing meaning in VBA terms. **Operand layouts and numbering are the established facts; stack effects and semantics are largely inferred from VBA language knowledge, and entries marked "inferred", "unverified" or "unknown" are guesses.** Operand layouts and numbering are high confidence unless said otherwise.
* Names below are this document's own labels (upper-case, category-first); they have no official standing and are not stored anywhere in the data.

## 1. Instruction word

### 1.1 Layout

Each instruction starts with one 16-bit word:

| Bits | Meaning |
|---|---|
| 0-9 | opcode number (0-1023), version-dependent numbering (section 2) |
| 10-15 | 6-bit "modifier" field, called MOD below. Its meaning depends on the opcode (section 3) |

There is no prefix/extension mechanism: every instruction is exactly one word followed by its operands. The operands follow immediately, in the order listed. Operand words are not aligned beyond the 2-byte granularity of the stream (all words are 2-byte multiples, so the stream stays even-aligned relative to a line start).

A line's instructions run from the line start offset until exactly the line's byte length is consumed. An opcode number that is not in the translation table for the build invalidates the remainder of that line, because operand counts are then unknown and the stream cannot be resynchronised.

### 1.2 Operand classes

| Class | Width | Notes |
|---|---|---|
| name | 2 | identifier reference; MOD supplies a type-suffix character (3.1) |
| W (plain) | 2 | number: counts, flags, raw literal words |
| obj | 2 | object-table reference: entry index times the pointer size |
| rec | 4 | offset into indirect table |
| ctx | 4 (x86) / 8 (x64) | see PROC_CONTEXT |

### 1.3 Variable-length trailer (V)

After all fixed operands: a 16-bit byte length N, then N bytes of data, then one zero pad byte if N is odd (so the next instruction starts on an even offset relative to the line start). Interpretation of the data depends on the opcode: text in the project code page (string literal, comments, re-parse text), a list of N/2 identifier-reference words (computed branches), or opaque bytes (line continuation). For comment instructions the fixed W operand precedes the length.

## 2. Opcode numbering

The tables in section 4 use a **canonical numbering** 0-263, which is the numbering stored in the module by the x64 build of VBA7. All other builds store a different numbering and must be translated to canonical before table lookup. The build is the project's platform kind (format document 2.1), not its version word.

| Producer (see format doc for how to detect) | Stored value `s` to canonical `c` |
|---|---|
| VBA6 and VBA7, 32-bit (x86) layout | `s` 0-173: c=s. 174-175: c=s+1. 176-178: c=s+2. 179-260: c=s+3 |
| VBA7 64-bit (x64) | c = s (identity) |
| VBA5 | `s` 0-68: c=s. 69-71: +1. 72-112: +3. 113-151: +7. 152-165: +8. 166-167: +9. 168-170: +10. 171-252: +11 |
| VBA3 (low confidence, see 6) | `s` 0-67: c=s. 68-70: +2. 71-111: +4. 112-150: +8. 151-164: +9. 165-166: +10. 167-169: +11. 170-238: +12. 239: c=263 |

Consequences, derived from the above:

* Canonical 174, 177 and 181 (the three 8-byte integer literals) **never occur** in x86 builds of any version.
* Canonical 116-119 (event declaration and raise) and 159 (Implements) never occur in VBA5 or VBA3 output; 73-74 (address-of) and 69 (assert) also never occur there.
* The x64 and x86 numberings differ only by the three 8-byte literal slots. **Verified**: a 32-bit and a 64-bit build of the same 25-module source decode to the same canonical opcode and modifier on all 1,021 lines with code, and 153 distinct opcodes (47 of them at or above 174) were decoded from 64-bit projects, 90 (21 at or above 174) from VBA7 32-bit ones, every line to its exact length. That the VBA6 and VBA7 x86 numberings are identical holds for the 21 distinct opcodes seen in VBA6 projects (6 of them in the shifted range 179-260: 185, 186, 212, 227, 248, 260); the +1 and +2 ranges (174-178) were not seen in VBA6. The VBA5 and VBA3 mappings are unverified (no project below version word 0x6B was available); VBA3 is the least certain (low confidence). The three 8-byte literals (174, 177, 181) were not seen in any 64-bit project.

## 3. MOD field usage

MOD (bits 10-15) is zero for most instructions. Where used:

### 3.1 Name operands: type suffix and bracketing
For any "name" operand, MOD selects a trailing declared-type character shown after the identifier:

| MOD | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 8 | 32 | other |
|---|---|---|---|---|---|---|---|---|---|---|
| Suffix | none | unknown | `%` | `&` | `!` | `#` | `@` | `$` | name shown in square brackets (an identifier that needed bracket quoting) | none |

MOD values 1, 7 and 9-13 have no known suffix meaning (unknown). Observed (VBA 7.0 x86): the suffix table held in every case seen (`$` 44 of 44, `&` 1 of 1, brackets 40 of 40); no name had a suffix that its MOD did not announce. Exceptions: for ERR_HANDLER_SET, MOD 1 = "Resume Next" form and MOD 2 = "GoTo 0" form (the name operand then carries no target); for ERR_RESUME, MOD 1 = "Resume Next" form, MOD 0 = resume at the given line or label name, any other non-zero MOD = no target.

### 3.2 Per-opcode MOD meanings

| Opcode(s) | MOD meaning |
|---|---|
| CONVERT_TO_TYPE, CONVERT_TO_TYPE_VARIANT, DEFTYPE_RANGE | target intrinsic type: 0 Variant, 2 Integer, 3 Long, 4 Single, 5 Double, 6 Currency, 7 Date, 8 String, 9 Object, 10 Error, 11 Boolean, 12 Variant, 17 Byte (1 unknown). Other values: unknown |
| DIM_MARKER, DIM_MARKER_IMPLICIT, TYPE_DECL | scope flags 0x04 Global, 0x08 Public, 0x10 Private, 0x20 Static (if several are set, the first in this list applies); flag 0x01 = Const (ignored for TYPE_DECL). Observed for TYPE_DECL: only 0 (no scope word) and 1 (`Private`) occur. Observed for the declaration instructions: bit 0x01 / 0x02 distinguish a variable from a constant (`Dim` vs. `Const`) |
| CONST_KEYWORD_VALUE | 0 False, 1 True, 2 Null, 3 Empty |
| INVOKE_SIMPLE, INVOKE_DOT, INVOKE_WITH_DOT | MOD < 16: statement written with the explicit `Call` keyword. MOD >= 16: no keyword; the real MOD is MOD-16 (that residual value then acts as the name-suffix selector, 3.1) |
| OPTION_STMT | 0 `Option Base 0`, 1 `Option Base 1`, 2 `Option Compare Text`, 3 `Option Compare Binary`, 4 `Option Explicit`, 5 `Option Private Module`; observed: 4 is `Option Explicit` and 7 is `Option Compare Database` |
| REDIM_SIMPLE, REDIM_SIMPLE_TYPED | bit 0x10 = `Preserve` |
| VAR_DECL | bit 0x20 = `WithEvents`; bit 0x10 = one extra trailing W follows the rec operand (its meaning is not determined; plausibly array dimension info; observed values 12 to 34); bits 0x01 and 0x02 distinguish a variable from a constant (`Dim` vs. `Const`), observed |
| PROC_HEADER and PROC_HEADER_ALT | bit 0x04 = `Public`; value 2 or 6 (after masking) = this is a Function rather than a Sub (see indirect-table function record). Observed values: 1 Private Sub, 2 Private Function (and every `Declare`), 5 Public Sub, 6 Public Function (also Property Get/Let) |

## 4. Opcode tables (canonical numbering)

Columns: `#` decimal canonical number; Name; Operands in order; Semantics/stack (inferred unless stated). "Pop n/push 1" is in terms of expression-stack values. A stack machine model is assumed: expression operands are evaluated first, then the operator instruction.

### 4.1 Logical and bitwise binary operators (pop 2, push 1)
| # | Name | Ops | Meaning |
|---|---|---|---|
| 0 | BOOL_IMPLICATION | - | `Imp` |
| 1 | BOOL_EQUIVALENCE | - | `Eqv` |
| 2 | BOOL_EXCLUSIVE_OR | - | `Xor` |
| 3 | BOOL_DISJUNCTION | - | `Or` |
| 4 | BOOL_CONJUNCTION | - | `And` |

### 4.2 Comparison (pop 2, push Boolean/Variant)
| # | Name | Ops | Meaning |
|---|---|---|---|
| 5 | COMPARE_EQUAL | - | `=` |
| 6 | COMPARE_NOT_EQUAL | - | `<>` |
| 7 | COMPARE_LESS_EQUAL | - | `<=` |
| 8 | COMPARE_GREATER_EQUAL | - | `>=` |
| 9 | COMPARE_LESS | - | `<` |
| 10 | COMPARE_GREATER | - | `>` |
| 18 | COMPARE_PATTERN | - | `Like` |
| 20 | COMPARE_OBJECT_IDENTITY | - | `Is` |

### 4.3 Arithmetic, string and unary operators
| # | Name | Ops | Meaning |
|---|---|---|---|
| 11 | NUM_ADD | - | `+` (pop 2, push 1) |
| 12 | NUM_SUBTRACT | - | `-` |
| 13 | NUM_REMAINDER | - | `Mod` |
| 14 | NUM_INT_DIVIDE | - | `\` |
| 15 | NUM_MULTIPLY | - | `*` |
| 16 | NUM_DIVIDE | - | `/` |
| 17 | TEXT_JOIN | - | `&` |
| 19 | NUM_POWER | - | `^` |
| 21 | BOOL_NEGATE | - | unary `Not` (pop 1, push 1) |
| 22 | NUM_NEGATE | - | unary minus |
| 23 | INTRINSIC_ABSOLUTE_VALUE | - | `Abs` |
| 24 | INTRINSIC_TRUNCATE | - | `Fix` |
| 25 | INTRINSIC_FLOOR | - | `Int` |
| 26 | INTRINSIC_SIGN | - | `Sgn` |
| 27 | INTRINSIC_LENGTH_CHARS | - | `Len` (character count) |
| 28 | INTRINSIC_LENGTH_BYTES | - | `LenB` |
| 29 | EXPR_PARENTHESES | - | parenthesised sub-expression marker (value unchanged) |
| 30 | EXPR_FILE_NUMBER_MARK | - | the `#` marker before a file number in I/O statements |

### 4.4 Variable, member, dictionary and index access
"READ" pushes a value; "WRITE" pops a value (and for member forms the target object beneath it) and assigns; "WRITE_REF" is `Set`-style reference assignment; "WITH" forms take the implicit With-block object instead of an explicit object operand on the stack. "PARAMS" forms carry a count W of arguments that were pushed before the instruction. "BANG" means the dictionary-style `!` access; "DOT" means `.` member access; "SIMPLE" means an unqualified name. Fixed operands: `name` for non-PARAMS forms; `name, W(argcount)` for PARAMS forms.

| # | Name | Ops | Meaning |
|---|---|---|---|
| 31 | READ_TARGET_NAME | name | load identifier used as assignment/loop target (left-hand-side form of a load) |
| 32 | READ_SIMPLE | name | push value of variable/function/constant; observed: the name word 0xFFFF is the implicit `Me` |
| 33 | READ_DOT | name | pop object; push `obj.name` (the receivers the lowering accepts: 4.18) |
| 34 | READ_BANG | name | pop object; push `obj!name` |
| 35 | READ_DEFAULT_INDEXED | W | pop object and W args; push default-member indexed result `obj(args)`; W = argument count (inferred) |
| 36 | READ_SIMPLE_PARAMS | name, W | call/index `name(args)` |
| 37 | READ_DOT_PARAMS | name, W | `obj.name(args)`; the W arguments are below the receiver, which is on top (4.18) |
| 38 | READ_BANG_PARAMS | name, W | `obj!name(args)` |
| 39 | WRITE_SIMPLE | name | assign |
| 40 | WRITE_DOT | name | `obj.name = v`; the value is below the receiver, which is on top (4.18) |
| 41 | WRITE_BANG | name | `obj!name = v` |
| 42 | WRITE_DEFAULT_INDEXED | W | `obj(args) = v` |
| 43 | WRITE_SIMPLE_PARAMS | name, W | `name(args) = v` |
| 44 | WRITE_DOT_PARAMS | name, W | `obj.name(args) = v` |
| 45 | WRITE_BANG_PARAMS | name, W | `obj!name(args) = v` |
| 46 | WRITE_REF_SIMPLE | name | `Set name = obj` |
| 47 | WRITE_REF_DOT | name | `Set obj.name = o` |
| 48 | WRITE_REF_BANG | name | `Set obj!name = o` |
| 49 | WRITE_REF_DEFAULT_INDEXED | W | `Set obj(args) = o` |
| 50 | WRITE_REF_SIMPLE_PARAMS | name, W | |
| 51 | WRITE_REF_DOT_PARAMS | name, W | |
| 52 | WRITE_REF_BANG_PARAMS | name, W | |
| 53 | READ_WITH_DOT | name | `.name` inside With |
| 54 | READ_WITH_BANG | name | `!name` inside With |
| 55 | READ_WITH_DOT_PARAMS | name, W | `.name(args)` |
| 56 | READ_WITH_BANG_PARAMS | name, W | `!name(args)` |
| 57 | WRITE_WITH_DOT | name | |
| 58 | WRITE_WITH_BANG | name | |
| 59 | WRITE_WITH_DOT_PARAMS | name, W | |
| 60 | WRITE_WITH_BANG_PARAMS | name, W | |
| 61 | WRITE_REF_WITH_DOT | name | |
| 62 | WRITE_REF_WITH_BANG | name | |
| 63 | WRITE_REF_WITH_DOT_PARAMS | name, W | |
| 64 | WRITE_REF_WITH_BANG_PARAMS | name, W | |
| 261 | WRITE_SIMPLE_AMBIGUOUS | name | assignment whose Let/Set nature is decided at run time or is ambiguous at compile time (inferred; low confidence) |

### 4.5 Calls
| # | Name | Ops | Meaning |
|---|---|---|---|
| 65 | INVOKE_SIMPLE | name, W | statement-level call of `name` with W pushed args; MOD (3.2) says whether `Call` keyword was written |
| 66 | INVOKE_DOT | name, W | `obj.name args` call statement; the W arguments are below the receiver, which is on top (4.18) |
| 67 | INVOKE_WITH_DOT | name, W | `.name args` inside With |
| 68 | ARRAY_ELEMENT_PARAMS | name, W | array element/arguments reference (inferred; unverified) |
| 73 | ADDRESS_OF_SIMPLE | name | `AddressOf name` |
| 74 | ADDRESS_OF_DOT | name | `AddressOf obj.name` |
| 210 | ARG_FORCE_BYVAL | - | next argument forced by value (parenthesised) |
| 211 | ARG_MISSING | - | an omitted positional argument, as in `F(a, , c)`; it takes the place of the value in the argument list (4.18) |
| 212 | ARG_NAMED | name | following argument is the named argument `name:=` |
| 171 | ARG_DEFAULT_PLACEHOLDER | - | default/missing argument placeholder (inferred) |

### 4.6 Event instructions
| # | Name | Ops | Meaning |
|---|---|---|---|
| 116 | DECLARE_EVENT | rec | `Event` declaration; rec points to a function record (6.3 of format doc) |
| 117 | EVENT_FIRE_SIMPLE | name, W | `RaiseEvent name(args)` |
| 118 | EVENT_FIRE_DOT | name, W | raise event through member |
| 119 | EVENT_FIRE_WITH_DOT | name, W | raise event through With object |

### 4.7 Literals
Multi-word literals are stored low word first (plain little-endian bytes of the value). Radix variants (decimal/hex/octal) record how the programmer wrote the literal; the numeric value semantics are identical.

| # | Name | Ops | Meaning |
|---|---|---|---|
| 169 | CONST_CURRENCY | W,W,W,W | 8-byte scaled integer Currency (value x 10000), push |
| 170 | CONST_DATE | W,W,W,W | 8-byte IEEE double date serial |
| 172 | CONST_INT16_DECIMAL | W | 16-bit signed integer, written in decimal |
| 173 | CONST_INT32_DECIMAL | W,W | 32-bit signed integer decimal |
| 174 | CONST_INT64_DECIMAL | W,W,W,W | 64-bit (LongLong) decimal. x64 only |
| 175 | CONST_INT16_HEX | W | hex-written 16-bit |
| 176 | CONST_INT32_HEX | W,W | hex-written 32-bit |
| 177 | CONST_INT64_HEX | 4 W | hex 64-bit, x64 only |
| 178 | CONST_NULL_OBJECT | - | `Nothing` |
| 179 | CONST_INT16_OCTAL | W | octal-written 16-bit |
| 180 | CONST_INT32_OCTAL | 2 W | octal 32-bit |
| 181 | CONST_INT64_OCTAL | 4 W | octal 64-bit, x64 only |
| 182 | CONST_FLOAT32 | 2 W | IEEE single |
| 183 | CONST_FLOAT64 | 4 W | IEEE double |
| 184 | CONST_SMALL_INT | - | small integer constant with no operand; the value is presumably carried in MOD (unverified, **unknown**) |
| 185 | CONST_TEXT | V | string literal text (project code page), see 1.3 |
| 186 | CONST_KEYWORD_VALUE | - | False/True/Null/Empty selected by MOD (3.2) |

### 4.8 Conversion and coercion
| # | Name | Ops | Meaning |
|---|---|---|---|
| 88 | CONVERT_TO_TYPE | - | convert top of stack to type in MOD |
| 89 | CONVERT_TO_TYPE_VARIANT | - | same, treating operand as a Variant (distinction not verified) |

### 4.9 Intrinsic functions with fixed argument count in the compiled form
| # | Name | Ops | Meaning |
|---|---|---|---|
| 125 | INTRINSIC_CURRENT_DIR | - | `CurDir` |
| 126 | INTRINSIC_DIR_LISTING | - | `Dir` |
| 127 | RESERVED_A | - | no operands; purpose unknown |
| 128 | RESERVED_B | - | no operands; purpose unknown |
| 129 | INTRINSIC_ERROR_TEXT | - | `Error[$]()` function |
| 130 | INTRINSIC_FORMAT_VALUE | - | `Format` |
| 131 | INTRINSIC_NEXT_FILE_NUMBER | - | `FreeFile` |
| 132 | INTRINSIC_SEARCH_A | - | `InStr` with 2 arguments |
| 133 | INTRINSIC_SEARCH_B | - | 3 arguments |
| 134 | INTRINSIC_SEARCH_C | - | 4 arguments |
| 135 | INTRINSIC_SEARCH_BYTES_A | - | byte version 2 args |
| 136 | INTRINSIC_SEARCH_BYTES_B | - | 3 args |
| 137 | INTRINSIC_SEARCH_BYTES_C | - | 4 args |
| 138 | INTRINSIC_LOWER_BOUND | W | `LBound`; W = argument form/count (inferred) |
| 139 | INTRINSIC_SUBSTRING | - | `Mid` (function) |
| 140 | INTRINSIC_SUBSTRING_BYTES | - | `MidB` |
| 141 | INTRINSIC_TEXT_COMPARE_A | - | `StrComp` 2-arg |
| 142 | INTRINSIC_TEXT_COMPARE_B | - | 3-arg |
| 143 | INTRINSIC_REPEAT_FROM_CODE | - | `String`(count, variant char) |
| 144 | INTRINSIC_REPEAT_FROM_TEXT | - | `String`(count, string) |
| 145 | INTRINSIC_UPPER_BOUND | W | `UBound` |

### 4.10 Statement and line markers
| # | Name | Ops | Meaning |
|---|---|---|---|
| 70 | STMT_BEGIN | W | beginning of a statement; W presumed to be a source column or indent; observed: consistent with the column of the statement start for values 32 and 66 in the sampled line, and the value 0 also occurs (partly verified) |
| 71 | STMT_BEGIN_IMPLICIT | - | statement start without explicit marker (e.g. after `Then`) |
| 72 | LINE_BEGIN | - | line beginning |
| 165 | DRAW_LINE | W | the `Line` graphics statement; W = option flags |
| 166 | LINE_CONTINUATION | V | `_` continuation; trailer: observed as pairs of 16-bit values, one per break: the number of tokens before the break and the indentation of the continuation line (292 of 318 pairs match both; the signed second value is negative inside comments) |
| 168 | LABEL_NUMERIC | name | numeric line label |
| 163 | LABEL_NAMED | name | named label |
| 227 | COMMENT_QUOTE | W, V | `'` comment; W = the column of the apostrophe in the line (observed, 2,636 of 2,640 comments), V = text |
| 231 | COMMENT_REM_KEYWORD | V | `Rem` comment text |
| 230 | TEXT_UNPARSED | V | raw text to be re-parsed (unparsed fragment, e.g. text that failed to compile) |

### 4.11 Flow control: conditionals and select
| # | Name | Ops | Meaning |
|---|---|---|---|
| 155 | IF_SINGLE_LINE | - | single-line `If` test; pop condition |
| 156 | IF_BLOCK_OPEN | - | block `If cond Then` |
| 99 | ELSE_SINGLE_LINE | - | single-line `Else` |
| 100 | BLOCK_ELSE_BRANCH | - | block `Else` |
| 101 | BLOCK_ELSEIF_BRANCH | - | `ElseIf cond Then` |
| 106 | ENDIF_SINGLE_LINE | - | end of single-line If |
| 107 | BLOCK_IF_CLOSE | - | `End If` |
| 157 | TYPE_TEST | obj | `TypeOf x Is Class` test; obj names class |
| 158 | IF_BLOCK_OPEN_TYPE_TEST | obj | block `If TypeOf ... Is ...` |
| 102 | BLOCK_ELSEIF_BRANCH_TYPE_TEST | obj | `ElseIf TypeOf ...` |
| 237 | SWITCH_BEGIN | - | `Select Case` expression start |
| 239 | SWITCH_BEGIN_TYPE | - | type-based select (inferred) |
| 238 | SWITCH_CASE_CLASS | obj | `Case Is <class>` object type case |
| 75 | CASE_VALUE | - | `Case x` |
| 76 | CASE_RANGE | - | `Case x To y` |
| 77 | CASE_IS_GREATER | - | `Case Is > x` |
| 78 | CASE_IS_LESS | - | `Case Is < x` |
| 79 | CASE_IS_GREATER_EQUAL | - | `Case Is >= x` |
| 80 | CASE_IS_LESS_EQUAL | - | `Case Is <= x` |
| 81 | CASE_IS_NOT_EQUAL | - | `Case Is <> x` |
| 82 | CASE_IS_EQUAL | - | `Case Is = x` |
| 83 | CASE_DEFAULT | - | `Case Else` |
| 84 | CASE_LIST_END | - | end of one case-list |
| 110 | SWITCH_END | - | `End Select` |

### 4.12 Loops and jumps
| # | Name | Ops | Meaning |
|---|---|---|---|
| 95 | DO_OPEN | - | `Do` |
| 97 | DO_OPEN_UNTIL | - | `Do Until cond` |
| 98 | DO_OPEN_WHILE | - | `Do While cond` |
| 188 | DO_CLOSE | - | `Loop` |
| 189 | DO_CLOSE_UNTIL | - | `Loop Until cond` |
| 190 | DO_CLOSE_WHILE | - | `Loop While cond` |
| 247 | WHILE_OPEN | - | `While cond` |
| 246 | WHILE_CLOSE | - | `Wend` |
| 146 | FOR_COUNTED | - | `For v = a To b` |
| 149 | FOR_COUNTED_STEP | - | `For ... Step` |
| 147 | FOR_COLLECTION | - | `For Each v In coll` |
| 148 | FOR_COLLECTION_TYPED | obj | `For Each v As Type In coll` |
| 202 | FOR_ADVANCE | - | `Next` |
| 203 | FOR_ADVANCE_NAMED | - | `Next v` |
| 257 | FOR_VAR_EXPR_CLOSE | - | end marker of for-variable expression |
| 258 | FOR_VAR_EXPR_OPEN | - | begin marker of for-variable expression |
| 120 | LEAVE_DO | - | |
| 121 | LEAVE_FOR | - | |
| 122 | LEAVE_FUNCTION | - | |
| 123 | LEAVE_PROPERTY | - | |
| 124 | LEAVE_SUB | - | |
| 153 | BRANCH_SUBROUTINE | name | |
| 154 | BRANCH_GOTO | name | `GoTo label`; lowered, differential run (4.18) |
| 233 | BRANCH_RETURN | - | `Return` |
| 205 | BRANCH_COMPUTED_SUBROUTINE | V | `On expr GoSub l1,l2..`; V holds N/2 identifier-reference words |
| 206 | BRANCH_COMPUTED_GOTO | V | `On expr GoTo ...`, same trailer |
| 248 | WITH_OPEN | - | `With` |
| 260 | WITH_EXPR_OPEN | - | begin of With object expression |
| 113 | WITH_CLOSE | - | `End With` |
| 103 | HALT_END | - | `End` statement; lowered, synthetic caches (4.18) |
| 242 | HALT_STOP | - | `Stop` |
| 96 | YIELD_EVENTS | - | `DoEvents` |
| 108 | IMMEDIATE_END | - | end of an immediate-window (non-module) statement sequence (inferred) |

### 4.13 Error handling
| # | Name | Ops | Meaning |
|---|---|---|---|
| 204 | ERR_HANDLER_SET | name | `On Error`: MOD 1 Resume Next; MOD 2 GoTo 0; else GoTo label `name` |
| 232 | ERR_RESUME | name | `Resume`: MOD 0 label/line, 1 `Resume Next`, other = bare `Resume` (no target) |
| 115 | ERR_RAISE_STMT | - | `Error n` statement |

### 4.14 Procedure, declaration and scope instructions
| # | Name | Ops | Meaning |
|---|---|---|---|
| 150 | PROC_HEADER | rec | `Sub`/`Function`/`Property` header; rec -> function record (format 6.3) |
| 151 | PROC_HEADER_ALT | rec | same layout, a variant (distinction unknown) |
| 105 | PROC_END_FUNCTION | - | `End Function` |
| 111 | PROC_END_SUB | - | `End Sub` |
| 109 | PROC_END_PROPERTY | - | `End Property` |
| 90 | PROC_CONTEXT | ctx | context record offset (D on x86; two D on x64; second D meaning unknown) |
| 104 | PROC_CONTEXT_END | - | end of context |
| 245 | VAR_DECL | rec [, W] | module/proc variable declaration; rec -> variable record (6.4). Extra W if MOD bit 0x10; the declaration lines of module-level variables and constants: 4.18 |
| 93 | DIM_MARKER | - | `Dim` marker, scope in MOD |
| 94 | DIM_MARKER_IMPLICIT | - | implicit declaration |
| 243 | TYPE_DECL | rec | `Type` header; rec -> record (6.5); scope in MOD |
| 112 | TYPE_DECL_END | - | `End Type` |
| 262 | ENUM_DECL_END | - | `End Enum` |
| 208 | OPTION_STMT | - | MOD selects option (3.2) |
| 209 | OPTION_BASE_MARKER | - | `Option Base` marker (relation to OPTION_STMT base values unknown) |
| 92 | DEFTYPE_RANGE | W, W | `DefInt A-Z` family: MOD = type; the two W are the first and last letters (inferred) |
| 159 | IMPLEMENTS_STMT | W,W | `Implements Interface`: two words (226 uses, all on x64, each line decodes to its exact length); the first is the position of the interface record in the indirect table plus 8 and differs for each interface, the record starts with the type descriptor of the interface (pcode-format.md 6.2a: 226 of 226 resolved), the second was 0 in every use. The 32-bit form was not seen |
| 194 | REDIM_DOT | name, W, rec | `ReDim obj.name(...)`; W = dimension count (inferred), rec type record |
| 195 | REDIM_DOT_WITH | name, W, rec | same inside With |
| 196 | REDIM_DOT_TYPED | name, W, rec | with `As Type` |
| 197 | REDIM_DOT_TYPED_WITH | name, W, rec | |
| 228 | REDIM_SIMPLE | name, W, rec | `ReDim name(...)` |
| 229 | REDIM_SIMPLE_TYPED | name, W, rec | with `As Type` |
| 259 | REDIM_NEW_MARKER | - | `ReDim ... As New` marker |
| 114 | ERASE_ARRAYS | W | `Erase`, W = count of arrays |
| 241 | FRAME_RESERVE | W, W | purpose unknown (stack/frame sizing, inferred) |
| 250 | CONST_EXPR_WITH_CALL | - | constant expression containing a function |
| 251 | PP_DEFINE | name | `#Const name` (conditional compilation, inferred) |
| 252 | PP_IF | - | `#If` |
| 253 | PP_ELSE | - | `#Else` |
| 254 | PP_ELSEIF | - | `#ElseIf` |
| 255 | PP_ENDIF | - | `#End If` |
| 256 | PP_MARKER | - | conditional-compilation marker (unknown details) |

### 4.15 Object lifetime and miscellaneous object statements
| # | Name | Ops | Meaning |
|---|---|---|---|
| 192 | SELF_REF | - | `Me` pushed |
| 193 | SELF_REF_IMPLICIT | - | implicit `Me` |
| 201 | NEW_OBJECT | obj | `New Class` |
| 240 | SET_KEYWORD_MARKER | - | `Set` keyword marker |
| 164 | LET_KEYWORD_MARKER | - | `Let` keyword marker |
| 91 | DEBUG_PREFIX | - | `Debug.` prefix marker |
| 69 | DEBUG_ASSERT | - | `Debug.Assert` |
| 263 | INVALID_SENTINEL | - | invalid instruction sentinel |

### 4.16 Strings statements and file I/O
| # | Name | Ops | Meaning |
|---|---|---|---|
| 198 | SUBSTR_ASSIGN | - | `Mid` statement (assignment form) |
| 199 | SUBSTR_BYTES_ASSIGN | - | `MidB` statement |
| 191 | LEFT_ALIGN_ASSIGN | - | `LSet` |
| 234 | RIGHT_ALIGN_ASSIGN | - | `RSet` |
| 200 | RENAME_FILE | - | `Name a As b` |
| 207 | FILE_OPEN | W | `Open`: W packs mode (low byte: 0x01 Input, 0x02 Output, 0x04 Random, 0x08 Append, 0x20 Binary), access (bits 8-11: 1 Read, 2 Write, 3 Read Write), lock (bits 12-15: 1 Lock Read Write, 2 Lock Write, 3 Lock Read, bit 4 set = Shared) |
| 86 | FILE_CLOSE | W | `Close`, W = channel count |
| 87 | FILE_CLOSE_ALL | - | `Close` with no channels |
| 160 | READ_LIST_OPEN | - | `Input #` |
| 161 | READ_LIST_CLOSE | - | |
| 162 | READ_LIST_ITEM | - | |
| 167 | FILE_READ_LINE | - | `Line Input #` |
| 152 | FILE_READ_RECORD | - | `Get` |
| 226 | FILE_WRITE_RECORD | - | `Put` |
| 187 | FILE_LOCK | - | `Lock` |
| 244 | FILE_UNLOCK | - | `Unlock` |
| 236 | FILE_SEEK | - | `Seek` |
| 213 | OUT_CHANNEL | - | `Print #` channel |
| 214 | OUT_COMMA | - | `,` separator |
| 215 | OUT_STATEMENT_END | - | end of print list |
| 216 | OUT_ITEM_THEN_COMMA | - | item followed by `,` |
| 217 | OUT_ITEM_LAST | - | last item (newline) |
| 218 | OUT_ITEM_THEN_SEMICOLON | - | item followed by `;` |
| 219 | OUT_NEWLINE | - | |
| 220 | OUT_TARGET_OBJECT | - | print to Debug-like object |
| 221 | OUT_SEMICOLON | - | |
| 222 | OUT_SPACES | - | `Spc(n)` |
| 223 | OUT_TAB | - | `Tab` |
| 224 | OUT_TAB_THEN_COMMA | - | `Tab(n),` |
| 249 | FILE_WRITE_CHANNEL | - | `Write #` |

### 4.17 Graphics / legacy statements
| # | Name | Ops | Meaning |
|---|---|---|---|
| 85 | DRAW_CIRCLE | W | `Circle`, W = option flags |
| 225 | DRAW_POINT | W | `PSet` |
| 235 | DRAW_SCALE | W | `Scale` |

### 4.18 Opcodes accepted by the lowering, and declaration lines

`vba-pcode-ir` lowers the opcodes below although `Opcode::evidence()` reports
`Inferred`, each only in the form given; every other form stays unsupported
and the report names the opcode. A form is accepted when hand-assembled caches
pin it (`tests/synthetic.rs`, `tests/classes.rs`) and, where the local project
uses it, when the differential run (the lowered cache program against the
program compiled from the original text, both on `vba-vm`; 871 runs and 112
whole-module sessions on the local project, 1,988 runs and 880 sessions on the
46 other projects, 0 mismatches) agrees. Receivers of the member forms are
described by their static type: a class of the project, a user-defined type, `Object`
or `Variant` (late bound), a built-in library object (`Err`, `Debug`, `Collection`;
early bound with the member id), and the names that qualify (a standard module, an
enum, the project, the class name of a class with a default instance, `VBA` and its
sub-libraries). "Procedures / instructions"
counts the lowered procedures of the local project that contain the opcode and
its occurrences in them.

| Opcode | Accepted form | Procedures / instructions | Verification level |
|---|---|---|---|
| ERR_HANDLER_SET | MOD 1 (`Resume Next`), MOD 0 with a label name (`GoTo label`), MOD 2 (`GoTo 0`) | 108 / 162 (all MODs) | differential for MOD 0 and 1; MOD 2 synthetic only (its one use in the local project is in a procedure that stays unsupported for another reason) |
| ERR_RESUME | MOD 0 with a label name | 56 / 131 | differential |
| LABEL_NAMED | a label definition | 57 / 114 | differential |
| CONST_INT16_HEX | an `Integer` hexadecimal literal | 0 / 0 in bodies; used by module-level constants | synthetic; constants: differential |
| CONST_DATE | a `Date` literal | 1 / 1 | differential |
| CONVERT_TO_TYPE | MOD 3 (`CLng`), MOD 8 (`CStr`) | 3 / 3 | differential |
| READ_DOT | the receivers listed above: `Me` (a variable, procedure or property of the module itself, and in a document or designer module any other name, bound late on the host base); a value of a class (a public field, a procedure or a property of the class); a value of a user-defined type (a field); a late-bound value; a built-in library object; a qualifying name. The receiver is on top of the stack and the result replaces it | 354 / 209 | differential, with a host base object that journals every access |
| READ_DOT_PARAMS | same receivers; the W arguments are pushed first, the receiver last | 47 / 30 | differential |
| INVOKE_DOT | same receivers, as a statement; MOD >= 16 is the form without `Call` | 69 / 50 | differential |
| WRITE_DOT | same receivers; the value is pushed first, the receiver last. The value is evaluated before the receiver | 115 / 70 | differential; swapping the two evaluations produces 5 mismatches |
| WRITE_DOT_PARAMS | a property with parameters of a class (`Property Let`), a late-bound member: the value is pushed first, then the arguments, then the receiver; the value is the last argument of the call | 7 / 1 | differential for the one procedure; synthetic |
| WRITE_REF_DOT | as WRITE_DOT, for `Set` (`Property Set`, a field of object type, `PutRef` on a late-bound member) | 103 / 64 | differential |
| WRITE_REF_SIMPLE | `Set v = x` on a local, a module variable, the function result or a `Property Set`; the source must be an object known at lowering time (a `Variant` source is checked at run time by a call that is not lowered) | 170 / 136 | differential |
| SET_KEYWORD_MARKER | marks a reference assignment; the next WRITE_REF_* consumes it | 273 / 192 | differential |
| NEW_OBJECT | the operand names a class module of the project (plain, or qualified with the project name) or the built-in `Collection`; the initializer and terminator of the class must be lowered | 61 / 60 | differential |
| TYPE_TEST | `TypeOf x Is T`, followed by `IF_BLOCK_OPEN` or `BLOCK_ELSEIF_BRANCH` | 1 / 1 | differential |
| ARG_NAMED | follows the value it names; the count of the call includes it | 7 / 6 | differential |
| DEBUG_ASSERT | `Debug.Assert cond` (the condition on the stack) | 4 / 4 | differential |
| FOR_COLLECTION | `For Each`, between FOR_VAR_EXPR_OPEN/CLOSE of the variable and the collection expression; `NEXT` is FOR_VAR_EXPR_OPEN, FOR_ADVANCE. Collections, `Object` and `Variant` only (a class may enumerate through an attribute that has no line in the cache) | 6 / 6 | differential |
| WITH_EXPR_OPEN, WITH_OPEN, WITH_CLOSE, READ_WITH_DOT, INVOKE_WITH_DOT (and the other `*_WITH_DOT` forms, which stand for the member form with the object of the block as receiver) | `With` on an object | 1 / 1 (READ_WITH_DOT 4 / 1) | differential for those; the other with-forms synthetic or not exercised |
| PP_MARKER (with PP_IF, PP_ELSE, PP_ENDIF, PP_DEFINE) | a conditional-compilation directive: the marker, the condition or value, and the closing instruction; no effect on the compiled branch. The lines of the branch not taken are TEXT_UNPARSED | 4 / 1 | differential; synthetic |
| ARG_MISSING | an omitted positional argument; it must meet an `Optional` parameter (project procedure, library or run-time function) or be passed on late bound | 14 / 22 | differential |

Opcodes that were `Verified` and are lowered for the first time: BRANCH_GOTO
(2 / 2, differential), HALT_END (`End`; synthetic only), and the `Const`
forms of VAR_DECL below.

Declaration lines `[obs]` (VBA 7.0 x86):

* A module-level variable or constant is one line `DIM_MARKER mod`, optionally
  a value expression, `VAR_DECL mod rec`. DIM_MARKER MOD (values 0, 1, 8, 16
  and 17 occur): 0x08 `Public`, 0x10 `Private`, neither = `Dim` (private),
  0x01 `Const`. VAR_DECL MOD & 3: 1 variable, 2 constant (0x10 adds the layout
  word, 3.2). Declarations between `TYPE_DECL` and `TYPE_DECL_END` are the
  fields of the type.
* For a constant, the instructions between DIM_MARKER and VAR_DECL are its
  value expression (this corrects the earlier note that the value is not
  located). In the local project 308 module-level constants have a single,
  optionally negated literal there and 53 a longer expression (names and
  operators), which the lowering does not generate. The literal keeps its type:
  the 32-bit decimal and hexadecimal forms are `Long` (written with `&`), the
  16-bit forms `Integer`.
* The type field of a constant without an `As` clause is empty or holds the
  intrinsic type of its value with the flag 0x40; neither is a declared type.
* Declarations that share a line (`Dim a As Long, b As String`) stand or fall
  together: the lowering generates all of them or none.
* A local `Const` has the same form inside a procedure; the lowering folds
  its value where the name is read and gives it no slot.

## 5. Operand shapes (for walking the stream without semantics)

Canonical numbers; everything not listed has no operands.

| Shape | Opcodes |
|---|---|
| name | 31, 32, 33, 34, 39, 40, 41, 46, 47, 48, 53, 54, 57, 58, 61, 62, 73, 74, 153, 154, 163, 168, 204, 212, 232, 251, 261 |
| name, W | 36, 37, 38, 43, 44, 45, 50, 51, 52, 55, 56, 59, 60, 63, 64, 65, 66, 67, 68, 117, 118, 119 |
| one W | 35, 42, 49, 70, 85, 86, 114, 138, 145, 165, 172, 175, 179, 207, 225, 235 |
| two W | 92, 159, 173, 176, 180, 182, 241 |
| four W | 169, 170, 174, 177, 181, 183 |
| one obj (W) | 102, 148, 157, 158, 201, 238 |
| one rec (D) | 116, 150, 151, 243 |
| rec (D), plus one W when MOD & 0x10 | 245 |
| ctx | 90 (D on x86; D, D on x64) |
| name, W, rec (D) | 194, 195, 196, 197, 228, 229 |
| V only | 166, 185, 205, 206, 230, 231 |
| W then V | 227 |

## 6. Uncertainty summary

* Numbers of operands and their widths: high confidence for all tabulated shapes (VBA6/7 x86 and x64), medium for VBA5 (opcode existence differs; operand shapes assumed shared), low for VBA3.
* Which canonical opcodes exist in which build: high confidence for the translation tables as they are algebraic.
* Semantics/stack effects: inferred, medium at best; entries marked "inferred", "unknown" or "unverified" are guesses.
* Unknown: purpose of RESERVED_A and RESERVED_B, CONST_SMALL_INT value location, the extra W of VAR_DECL with MOD 0x10, the second word of IMPLEMENTS_STMT, the second D of PROC_CONTEXT on x64, difference between PROC_HEADER and PROC_HEADER_ALT, between CONVERT_TO_TYPE and CONVERT_TO_TYPE_VARIANT, OPTION_STMT vs OPTION_BASE_MARKER, PP_MARKER.
