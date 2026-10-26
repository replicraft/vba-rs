' INV: vbal.5.4.5.5, stmt.lock-unlock
' SOURCE: spec ms-vbal 5.4.5.5 Unlock range must be identical to the locked range
' EXPECT: err=True
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Open "/t/k.dat" For Random As #1 Len = 4
    Lock #1, 1 To 3
    On Error Resume Next
    Unlock #1, 1 To 2
    Debug.Print "err=" & CStr(Err.Number <> 0)
End Sub
