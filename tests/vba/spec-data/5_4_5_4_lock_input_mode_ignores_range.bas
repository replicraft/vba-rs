' INV: vbal.5.4.5.4, stmt.lock-unlock
' SOURCE: spec ms-vbal 5.4.5.4 for Input, Output, Append the range is treated as absent
' EXPECT: err=0
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Open "/t/k.txt" For Output As #1
    Print #1, "x"
    Close #1
    Open "/t/k.txt" For Input As #1
    On Error Resume Next
    Lock #1, 1 To 3
    Unlock #1
    Debug.Print "err=" & CStr(Err.Number)
End Sub
