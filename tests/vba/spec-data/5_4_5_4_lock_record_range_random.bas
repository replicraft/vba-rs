' INV: vbal.5.4.5.4, stmt.lock-unlock
' SOURCE: spec ms-vbal 5.4.5.4 record ranges on a Random file; Unlock with identical range
' XFAIL: wrong not implemented: tracking of Lock/Unlock ranges
' EXPECT: err=0
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Open "/t/k.dat" For Random As #1 Len = 4
    On Error Resume Next
    Lock #1, 2
    Lock #1, 3 To 5
    Lock #1, To 4
    Unlock #1, 2
    Unlock #1, 3 To 5
    Unlock #1, To 4
    Debug.Print "err=" & CStr(Err.Number)
End Sub
