' INV: vbal.5.4.5.5, stmt.lock-unlock
' SOURCE: spec ms-vbal 5.4.5.4 a lock is removed by an Unlock with the same range
' XFAIL: wrong pending
' EXPECT: first=0
' EXPECT: second=True
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Open "/t/k.dat" For Random As #1 Len = 4
    Lock #1, 2 To 3
    On Error Resume Next
    Unlock #1, 2 To 3
    Debug.Print "first=" & CStr(Err.Number)
    Unlock #1, 2 To 3
    Debug.Print "second=" & CStr(Err.Number <> 0)
End Sub
