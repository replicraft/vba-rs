' INV: vbal.5.4.5.5, stmt.lock-unlock
' SOURCE: spec ms-vbal 5.4.5.4 Close removes all locks of its file number
' EXPECT: err=True
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Open "/t/k.dat" For Random As #1 Len = 4
    Lock #1, 1 To 3
    Close #1
    Open "/t/k.dat" For Random As #1 Len = 4
    On Error Resume Next
    Unlock #1, 1 To 3
    Debug.Print "err=" & CStr(Err.Number <> 0)
End Sub
