' INV: vbal.5.4.5.2, stmt.close
' SOURCE: spec ms-vbal 5.4.5.2 Close of one number leaves other numbers open
' EXPECT: still open
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Open "/t/a" For Output As #1
    Open "/t/b" For Output As #2
    Close #1
    Print #2, "x"
    Close #2
    Debug.Print "still open"
End Sub
