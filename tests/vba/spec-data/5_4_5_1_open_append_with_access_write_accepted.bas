' INV: vbal.5.4.5.1, stmt.open
' SOURCE: spec ms-vbal 5.4.5.1 Append accepts Access Write and Access Read Write
' XFAIL: wrong pending
' EXPECT: ok
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Open "/t/a1.txt" For Append Access Write As #1
    Print #1, "x"
    Close #1
    Open "/t/a2.txt" For Append Access Read Write As #1
    Print #1, "x"
    Close #1
    Debug.Print "ok"
End Sub
