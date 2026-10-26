' INV: vbal.5.4.5.1, stmt.print
' SOURCE: spec ms-vbal 5.4.5.1 Print # is not valid on a file opened for Input
' EXPECT: err=True
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Open "/t/m.txt" For Output As #1
    Print #1, "x"
    Close #1
    Open "/t/m.txt" For Input As #1
    On Error Resume Next
    Print #1, "y"
    Debug.Print "err=" & CStr(Err.Number <> 0)
End Sub
