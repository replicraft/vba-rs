' INV: vbal.5.4.5.4, stmt.lock-unlock
' SOURCE: spec ms-vbal 5.4.5.4 Lock without a range locks the whole file
' EXPECT: err=0
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Open "/t/k.dat" For Random As #1 Len = 4
    On Error Resume Next
    Lock #1
    Unlock #1
    Debug.Print "err=" & CStr(Err.Number)
End Sub
