' INV: vbal.5.4.5.1, stmt.put
' SOURCE: spec ms-vbal 5.4.5.1 Put # is not valid on a file opened for Input
' EXPECT: err=True
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim n As Long
    Open "/t/m.txt" For Output As #1
    Print #1, "x"
    Close #1
    Open "/t/m.txt" For Input As #1
    On Error Resume Next
    Put #1, 1, n
    Debug.Print "err=" & CStr(Err.Number <> 0)
End Sub
