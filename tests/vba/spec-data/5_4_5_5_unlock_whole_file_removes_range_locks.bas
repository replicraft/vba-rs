' INV: vbal.5.4.5.5, stmt.lock-unlock
' SOURCE: spec ms-vbal 5.4.5.5 Unlock without range removes all restrictions; later range unlock has no match
' EXPECT: err=True
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Open "/t/k.dat" For Random As #1 Len = 4
    Lock #1, 1 To 3
    Unlock #1
    On Error Resume Next
    Unlock #1, 1 To 3
    Debug.Print "err=" & CStr(Err.Number <> 0)
End Sub
