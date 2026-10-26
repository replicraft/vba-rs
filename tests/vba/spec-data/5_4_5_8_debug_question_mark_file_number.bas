' INV: vbal.5.4.5.8, stmt.print
' SOURCE: spec ms-vbal 5.4.5.8 revision 3.0 grammar: "?" form with a marked file number
' XFAIL: wrong pending
' EXPECT: [viaQ]
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim s As String
    Open "/t/p.txt" For Output As #1
    Debug.? #1, "viaQ"
    Close #1
    Open "/t/p.txt" For Input As #1
    Line Input #1, s
    Close #1
    Debug.Print "[" & s & "]"
End Sub
