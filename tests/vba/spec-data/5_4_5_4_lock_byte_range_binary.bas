' INV: vbal.5.4.5.4, stmt.lock-unlock
' SOURCE: spec ms-vbal 5.4.5.4 byte ranges on a Binary file
' EXPECT: err=0
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Open "/t/k.bin" For Binary As #1
    On Error Resume Next
    Lock #1, 1 To 10
    Unlock #1, 1 To 10
    Debug.Print "err=" & CStr(Err.Number)
End Sub
