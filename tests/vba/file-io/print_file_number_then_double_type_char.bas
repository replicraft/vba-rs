' INV: stmt.print, lit.date
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/print-statement a file number followed by a Double literal with the # type character is not a date literal
' GRANT: file-read
' GRANT: file-write
' EXPECT: [ 5 ]
Public Sub Main()
    MkDir "/t"
    Dim s As String
    Open "/t/d.txt" For Output As #1
    Print #1, 5#
    Close #1
    Open "/t/d.txt" For Input As #1
    Line Input #1, s
    Close #1
    Debug.Print "[" & s & "]"
End Sub
