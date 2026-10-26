' INV: vbal.5.4.5.2, stmt.close
' SOURCE: spec ms-vbal 5.4.5.2 Close with a number list closes each named file
' EXPECT: closed
' ERROR: 52
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Open "/t/a" For Output As #1
    Open "/t/b" For Output As #2
    Close #1, #2
    Debug.Print "closed"
    Print #2, "x"
End Sub
