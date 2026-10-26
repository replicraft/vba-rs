' INV: stmt.print, lit.date
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/print-statement a file number followed by a Double literal with the # type character is not a date literal
' GRANT: file-read
' GRANT: file-write
' EXPECT: [ 5 ]
Public Sub Main()
    MkDir "/t"
    Dim dec As Integer, s As String
    dec = 3
    Open "/t/d.txt" For Output As #dec
    Print #dec, 5#
    Close #dec
    Open "/t/d.txt" For Input As #dec
    Line Input #dec, s
    Close #dec
    Debug.Print "[" & s & "]"
End Sub
