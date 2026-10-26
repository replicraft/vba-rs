' INV: vbal.5.4.5.4, stmt.lock-unlock
' SOURCE: spec ms-vbal 5.4.5.4 start record must be at least 1
' EXPECT: err=True
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Open "/t/k.dat" For Random As #1 Len = 4
    On Error Resume Next
    Lock #1, 0 To 2
    Debug.Print "err=" & CStr(Err.Number <> 0)
End Sub
