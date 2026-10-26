' INV: stmt.print, lit.date
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/print-statement a file number followed by a Double literal with the # type character is not a date literal
' GRANT: file-read
' GRANT: file-write
' EXPECT: [ 123456789012345 ]
Public Sub Main()
    MkDir "/t"
    Dim s As String
    Open "/t/d.txt" For Output As #255
    Print #255, 123456789012345#
    Close #255
    Open "/t/d.txt" For Input As #255
    Line Input #255, s
    Close #255
    Debug.Print "[" & s & "]"
End Sub
