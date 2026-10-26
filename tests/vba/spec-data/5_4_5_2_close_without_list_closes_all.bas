' INV: vbal.5.4.5.2, stmt.close
' SOURCE: spec ms-vbal 5.4.5.2 Close without a list closes every file
' EXPECT: closed
' ERROR: 52
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Open "/t/a" For Output As #1
    Open "/t/b" For Output As #300
    Close
    Debug.Print "closed"
    Print #300, "x"
End Sub
