' INV: vbal.5.4.5.1, stmt.seek, stmt.width, stmt.lock-unlock
' SOURCE: spec ms-vbal 5.4.5.1 Seek, Width, Lock, Unlock are valid on a file opened for Input
' EXPECT: err=0
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Open "/t/m.txt" For Output As #1
    Print #1, "x"
    Close #1
    Open "/t/m.txt" For Input As #1
    On Error Resume Next
    Seek #1, 1
    Width #1, 80
    Lock #1
    Unlock #1
    Debug.Print "err=" & CStr(Err.Number)
End Sub
