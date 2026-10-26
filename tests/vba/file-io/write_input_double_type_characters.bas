' INV: stmt.write, stmt.input, lit.date
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/print-statement a file number followed by a Double literal with the # type character is not a date literal
' GRANT: file-read
' GRANT: file-write
' EXPECT: 3
Public Sub Main()
    MkDir "/t"
    Dim a As Double, b As Double
    Open "/t/d.txt" For Output As #1
    Write #1, 1#, 2#
    Close #1
    Open "/t/d.txt" For Input As #1
    Input #1, a, b
    Close #1
    Debug.Print CStr(a + b)
End Sub
