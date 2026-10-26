//! File statements and file system functions over the in-memory file
//! system: text, binary and random access, directory functions and errors.

mod common;

use common::*;

#[test]
fn sequential_text() {
    check_files(&[
        fcase(
            "print_text_and_numbers",
            "Sub Main()\nDim f As Integer\nf = FreeFile\nOpen \"out.txt\" For Output As #f\nPrint #f, \"hello\"\nPrint #f, 42; \"x\"; -1.5\nPrint #f, \"a\", \"b\"\nClose #f\nEnd Sub\n",
            &[],
            "",
            &[("/out.txt", "hello\r\n 42 x-1.5 \r\na             b\r\n")],
        ),
        fcase(
            "print_without_items_is_a_blank_line",
            "Sub Main()\nOpen \"o.txt\" For Output As #1\nPrint #1,\nPrint #1, \"x\";\nPrint #1, \"y\"\nClose\nEnd Sub\n",
            &[],
            "",
            &[("/o.txt", "\r\nxy\r\n")],
        ),
        fcase(
            "print_tab_and_spc",
            "Sub Main()\nOpen \"o.txt\" For Output As #1\nPrint #1, Tab(5); \"x\"; Spc(2); \"y\"\nClose #1\nEnd Sub\n",
            &[],
            "",
            &[("/o.txt", "    x  y\r\n")],
        ),
        fcase(
            "width_wraps_between_items",
            "Sub Main()\nOpen \"o.txt\" For Output As #1\nWidth #1, 10\nPrint #1, \"abcdef\"; \"ghijkl\"\nClose #1\nEnd Sub\n",
            &[],
            "",
            &[("/o.txt", "abcdef\r\nghijkl\r\n")],
        ),
        fcase(
            "write_statement_quotes_and_formats",
            "Sub Main()\nDim d As Date\nd = DateSerial(2024, 3, 5)\nOpen \"w.txt\" For Output As #1\nWrite #1, \"a\", 1, True, 2.5\nWrite #1, Null, d\nWrite #1, \"x\"; 5\nWrite #1,\nClose #1\nEnd Sub\n",
            &[],
            "",
            &[(
                "/w.txt",
                "\"a\",1,#TRUE#,2.5\r\n#NULL#,#2024-03-05#\r\n\"x\",5\r\n\r\n",
            )],
        ),
        fcase(
            "line_input_handles_all_line_ends",
            "Sub Main()\nDim s As String\nOpen \"in.txt\" For Input As #1\nDo While Not EOF(1)\nLine Input #1, s\nDebug.Print \"[\" & s & \"]\"\nLoop\nClose #1\nEnd Sub\n",
            &[("in.txt", "one\r\ntwo\nthree\r\n\r\nlast")],
            "[one]\n[two]\n[three]\n[]\n[last]\n",
            &[],
        ),
        fcase(
            "write_then_input_round_trip",
            "Sub Main()\nDim a As String, b As Long, c As Double, d As Boolean\nOpen \"d.txt\" For Output As #1\nWrite #1, \"text, with comma\", 7, 2.5, True\nClose #1\nOpen \"d.txt\" For Input As #1\nInput #1, a, b, c, d\nClose #1\nDebug.Print a & \"|\" & b & \"|\" & c & \"|\" & d\nEnd Sub\n",
            &[],
            "text, with comma|7|2.5|True\n",
            &[],
        ),
        fcase(
            "input_reads_across_lines_and_trims",
            "Sub Main()\nDim x As Long, y As Long, z As Long, w As Long\nOpen \"n.txt\" For Input As #1\nInput #1, x, y, z\nInput #1, w\nClose #1\nDebug.Print x & y & z & w\nEnd Sub\n",
            &[("n.txt", "1, 2,3\r\n4\r\n")],
            "1234\n",
            &[],
        ),
        fcase(
            "input_into_variants",
            "Sub Main()\nDim a As Variant, b As Variant, c As Variant, d As Variant\nOpen \"v.txt\" For Input As #1\nInput #1, a, b, c, d\nClose #1\nDebug.Print TypeName(a) & TypeName(b) & TypeName(c) & TypeName(d)\nEnd Sub\n",
            &[("v.txt", "\"s\",12,1.5,#TRUE#\r\n")],
            "StringIntegerDoubleBoolean\n",
            &[],
        ),
        fcase(
            "append_adds_at_the_end",
            "Sub Main()\nOpen \"log.txt\" For Append As #1\nPrint #1, \"b\"\nClose #1\nEnd Sub\n",
            &[("log.txt", "a\r\n")],
            "",
            &[("/log.txt", "a\r\nb\r\n")],
        ),
        fcase(
            "output_truncates",
            "Sub Main()\nOpen \"log.txt\" For Output As #1\nPrint #1, \"new\"\nClose #1\nEnd Sub\n",
            &[("log.txt", "old old old\r\n")],
            "",
            &[("/log.txt", "new\r\n")],
        ),
        fcase(
            "eof_lof_loc_seek",
            "Sub Main()\nDim s As String\nOpen \"in.txt\" For Input As #1\nDebug.Print EOF(1) & LOF(1) & Seek(1)\nLine Input #1, s\nDebug.Print EOF(1) & Seek(1)\nSeek #1, 1\nLine Input #1, s\nDebug.Print s\nClose #1\nEnd Sub\n",
            &[("in.txt", "ab\r\ncd")],
            "False61\nFalse5\nab\n",
            &[],
        ),
        fcase(
            "file_numbers_and_freefile",
            "Sub Main()\nDim a As Integer, b As Integer\na = FreeFile\nOpen \"x.txt\" For Output As #a\nb = FreeFile\nDebug.Print a & b\nClose #a\nDebug.Print FreeFile\nEnd Sub\n",
            &[],
            "12\n 1 \n",
            &[],
        ),
        fcase(
            "close_without_arguments_closes_everything",
            "Sub Main()\nOpen \"a.txt\" For Output As #1\nOpen \"b.txt\" For Output As #2\nPrint #1, \"a\"\nPrint #2, \"b\"\nClose\nDebug.Print FreeFile\nEnd Sub\n",
            &[],
            " 1 \n",
            &[("/a.txt", "a\r\n"), ("/b.txt", "b\r\n")],
        ),
        fcase(
            "end_closes_and_flushes_open_files",
            "Sub Main()\nOpen \"a.txt\" For Output As #1\nPrint #1, \"kept\"\nEnd\nEnd Sub\n",
            &[],
            "",
            &[("/a.txt", "kept\r\n")],
        ),
        fcase(
            "windows_style_paths",
            "Sub Main()\nMkDir \"C:\\Data\"\nOpen \"C:\\Data\\f.txt\" For Output As #1\nPrint #1, \"w\"\nClose #1\nDebug.Print Dir(\"c:\\data\\*.txt\")\nEnd Sub\n",
            &[],
            "f.txt\n",
            &[("c:/Data/f.txt", "w\r\n")],
        ),
        fcase(
            "windows_1252_text",
            "Sub Main()\nOpen \"e.txt\" For Output As #1\nPrint #1, ChrW(233)\nClose #1\nDim s As String\nOpen \"e.txt\" For Input As #1\nLine Input #1, s\nClose #1\nDebug.Print AscW(s)\nEnd Sub\n",
            &[],
            " 233 \n",
            &[],
        ),
    ]);
}

#[test]
fn binary_and_random() {
    check_files(&[
        fcase(
            "binary_put_get",
            "Sub Main()\nDim n As Long, s As String * 5, x As Long, t As String * 5\nOpen \"b.bin\" For Binary As #1\nn = 258\nPut #1, , n\ns = \"hello\"\nPut #1, , s\nSeek #1, 1\nGet #1, , x\nGet #1, , t\nDebug.Print x & t & LOF(1)\nClose #1\nEnd Sub\n",
            &[],
            "258hello9\n",
            &[("/b.bin", "\u{2}\u{1}\0\0hello")],
        ),
        fcase(
            "binary_position_argument",
            "Sub Main()\nDim b As Byte\nOpen \"b.bin\" For Binary As #1\nGet #1, 3, b\nDebug.Print b\nPut #1, 1, b\nClose #1\nEnd Sub\n",
            &[("b.bin", "abc")],
            " 99 \n",
            &[("/b.bin", "cbc")],
        ),
        fcase(
            "binary_variable_length_string_uses_current_length",
            "Sub Main()\nDim s As String\nOpen \"b.bin\" For Binary As #1\ns = \"xyz\"\nGet #1, 2, s\nClose #1\nDebug.Print s\nEnd Sub\n",
            &[("b.bin", "abcdef")],
            "bcd\n",
            &[],
        ),
        fcase(
            "random_udt_records",
            "Private Type Rec\nId As Long\nName As String * 6\nEnd Type\nSub Main()\nDim r As Rec, i As Long\nOpen \"r.dat\" For Random As #1 Len = 10\nFor i = 1 To 3\nr.Id = i * 10\nr.Name = \"n\" & i\nPut #1, i, r\nNext\nGet #1, 2, r\nDebug.Print r.Id & \"|\" & Trim(r.Name) & \"|\" & LOF(1) & \"|\" & Loc(1)\nClose #1\nEnd Sub\n",
            &[],
            "20|n2|30|2\n",
            &[],
        ),
        fcase(
            "random_sequential_get_advances",
            "Sub Main()\nDim a As Long, b As Long\nOpen \"r.dat\" For Random As #1 Len = 4\nPut #1, 1, 11\nPut #1, 2, 22\nGet #1, 1, a\nGet #1, , b\nClose #1\nDebug.Print a & b\nEnd Sub\n",
            &[],
            "1122\n",
            &[],
        ),
        fcase(
            "random_variant_and_variable_string",
            "Sub Main()\nDim v As Variant, s As String\nOpen \"r.dat\" For Random As #1 Len = 32\nv = \"text\"\nPut #1, 1, v\nPut #1, 2, \"abc\"\nv = Empty\nGet #1, 1, v\ns = \"\"\nGet #1, 2, s\nClose #1\nDebug.Print v & \"|\" & s\nEnd Sub\n",
            &[],
            "text|abc\n",
            &[],
        ),
        fcase(
            "put_array_of_integers",
            "Sub Main()\nDim a(1 To 3) As Integer, b(1 To 3) As Integer, i As Long\nFor i = 1 To 3\na(i) = i * 100\nNext\nOpen \"a.bin\" For Binary As #1\nPut #1, , a\nSeek #1, 1\nGet #1, , b\nClose #1\nDebug.Print b(1) + b(2) + b(3)\nEnd Sub\n",
            &[],
            " 600 \n",
            &[],
        ),
    ]);
}

#[test]
fn directories_and_file_functions() {
    check_files(&[
        fcase(
            "dir_lists_matches_in_order",
            "Sub Main()\nDim f As String\nf = Dir(\"*.txt\")\nDo While f <> \"\"\nDebug.Print f\nf = Dir()\nLoop\nEnd Sub\n",
            &[("b.txt", ""), ("a.txt", ""), ("c.dat", "")],
            "a.txt\nb.txt\n",
            &[],
        ),
        fcase(
            "dir_of_a_single_file_tests_existence",
            "Sub Main()\nDebug.Print (Dir(\"x.txt\") <> \"\") & (Dir(\"nope.txt\") <> \"\")\nEnd Sub\n",
            &[("x.txt", "")],
            "TrueFalse\n",
            &[],
        ),
        fcase(
            "dir_with_directory_attribute",
            "Sub Main()\nMkDir \"d\"\nDebug.Print Dir(\"*\", vbDirectory)\nDebug.Print Dir(\"*\")\nEnd Sub\n",
            &[],
            "d\n\n",
            &[],
        ),
        fcase(
            "kill_wildcard",
            "Sub Main()\nKill \"*.txt\"\nDebug.Print Dir(\"*\")\nEnd Sub\n",
            &[("a.txt", ""), ("b.txt", ""), ("c.dat", "")],
            "c.dat\n",
            &[],
        ),
        fcase(
            "mkdir_chdir_name_rmdir",
            "Sub Main()\nMkDir \"sub\"\nOpen \"sub/f.txt\" For Output As #1\nClose #1\nChDir \"sub\"\nDebug.Print CurDir\nDebug.Print Dir(\"f.txt\")\nName \"f.txt\" As \"g.txt\"\nDebug.Print Dir(\"g.txt\")\nKill \"g.txt\"\nChDir \"..\"\nRmDir \"sub\"\nDebug.Print Dir(\"sub\", vbDirectory) = \"\"\nEnd Sub\n",
            &[],
            "/sub\nf.txt\ng.txt\nTrue\n",
            &[],
        ),
        fcase(
            "filecopy_filelen_attributes_datetime",
            "Sub Main()\nFileCopy \"x.txt\", \"y.txt\"\nDebug.Print FileLen(\"y.txt\")\nSetAttr \"y.txt\", vbReadOnly\nDebug.Print GetAttr(\"y.txt\") And vbReadOnly\nDebug.Print Format(FileDateTime(\"x.txt\"), \"yyyy-mm-dd\")\nSetAttr \"y.txt\", vbNormal\nKill \"y.txt\"\nDebug.Print Dir(\"y.txt\") = \"\"\nEnd Sub\n",
            &[("x.txt", "hello")],
            " 5 \n 1 \n2000-01-01\nTrue\n",
            &[("/x.txt", "hello")],
        ),
        fcase(
            "name_moves_between_directories",
            "Sub Main()\nMkDir \"d\"\nName \"a.txt\" As \"d/b.txt\"\nDebug.Print Dir(\"a.txt\") = \"\"\nDebug.Print Dir(\"d/b.txt\")\nEnd Sub\n",
            &[("a.txt", "z")],
            "True\nb.txt\n",
            &[("/d/b.txt", "z")],
        ),
        fcase(
            "relative_paths_follow_chdir",
            "Sub Main()\nChDir \"data\"\nOpen \"f.txt\" For Input As #1\nDim s As String\nLine Input #1, s\nClose #1\nDebug.Print s\nEnd Sub\n",
            &[("data/f.txt", "inside")],
            "inside\n",
            &[],
        ),
    ]);
}

#[test]
fn file_errors() {
    check_files(&[
        fcase(
            "errors_have_vba_numbers",
            "Function Try(code As String) As String\nEnd Function\nSub Main()\nOn Error Resume Next\nDim s As String, x As Long\nOpen \"missing.txt\" For Input As #1\nDebug.Print \"open missing \" & Err.Number: Err.Clear\nOpen \"a.txt\" For Output As #1\nOpen \"a.txt\" For Output As #2\nDebug.Print \"open twice \" & Err.Number: Err.Clear\nOpen \"b.txt\" For Output As #1\nDebug.Print \"number in use \" & Err.Number: Err.Clear\nClose\nOpen \"b.txt\" For Output As #600\nDebug.Print \"bad number \" & Err.Number: Err.Clear\nPrint #7, \"x\"\nDebug.Print \"not open \" & Err.Number: Err.Clear\nOpen \"a.txt\" For Input As #1\nLine Input #1, s\nLine Input #1, s\nDebug.Print \"past end \" & Err.Number: Err.Clear\nClose\nOpen \"a.txt\" For Output As #1\nLine Input #1, s\nDebug.Print \"wrong mode \" & Err.Number: Err.Clear\nGet #1, , x\nDebug.Print \"get on text \" & Err.Number: Err.Clear\nClose\nOpen \"r.dat\" For Random As #1 Len = 4\nGet #1, 0, x\nDebug.Print \"record 0 \" & Err.Number: Err.Clear\nGet #1, 5, x\nDebug.Print \"record past end \" & Err.Number: Err.Clear\nPut #1, 1, \"too long for four\"\nDebug.Print \"record length \" & Err.Number\nEnd Sub\n",
            &[("a.txt", "only")],
            "open missing 53\nopen twice 55\nnumber in use 55\nbad number 52\nnot open 52\npast end 62\nwrong mode 54\nget on text 54\nrecord 0 63\nrecord past end 62\nrecord length 59\n",
            &[],
        ),
        fcase(
            "directory_function_errors",
            "Sub Main()\nOn Error Resume Next\nMkDir \"d\"\nMkDir \"d\"\nDebug.Print \"mkdir twice \" & Err.Number: Err.Clear\nMkDir \"x/y\"\nDebug.Print \"mkdir no parent \" & Err.Number: Err.Clear\nRmDir \"nope\"\nDebug.Print \"rmdir missing \" & Err.Number: Err.Clear\nOpen \"d/f\" For Output As #1\nClose\nRmDir \"d\"\nDebug.Print \"rmdir full \" & Err.Number: Err.Clear\nChDir \"nope\"\nDebug.Print \"chdir missing \" & Err.Number: Err.Clear\nKill \"nope.txt\"\nDebug.Print \"kill missing \" & Err.Number: Err.Clear\nName \"nope\" As \"other\"\nDebug.Print \"name missing \" & Err.Number: Err.Clear\nName \"a.txt\" As \"b.txt\"\nDebug.Print \"name exists \" & Err.Number: Err.Clear\nFileCopy \"nope\", \"z\"\nDebug.Print \"copy missing \" & Err.Number: Err.Clear\nDebug.Print FileLen(\"nope\")\nDebug.Print \"filelen missing \" & Err.Number\nEnd Sub\n",
            &[("a.txt", "1"), ("b.txt", "2")],
            "mkdir twice 75\nmkdir no parent 76\nrmdir missing 76\nrmdir full 75\nchdir missing 76\nkill missing 53\nname missing 53\nname exists 58\ncopy missing 53\nfilelen missing 53\n",
            &[],
        ),
        fcase(
            "read_only_files_cannot_be_overwritten_or_deleted",
            "Sub Main()\nOn Error Resume Next\nSetAttr \"ro.txt\", vbReadOnly\nOpen \"ro.txt\" For Output As #1\nDebug.Print Err.Number: Err.Clear\nKill \"ro.txt\"\nDebug.Print Err.Number\nEnd Sub\n",
            &[("ro.txt", "keep")],
            " 70 \n 70 \n",
            &[("/ro.txt", "keep")],
        ),
        fcase(
            "killing_an_open_file_is_error_55",
            "Sub Main()\nOn Error Resume Next\nOpen \"a.txt\" For Input As #1\nKill \"a.txt\"\nDebug.Print Err.Number\nEnd Sub\n",
            &[("a.txt", "x")],
            " 55 \n",
            &[],
        ),
    ]);
}
