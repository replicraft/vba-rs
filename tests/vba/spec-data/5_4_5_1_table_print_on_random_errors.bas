' INV: vbal.5.4.5.1, stmt.print
' SOURCE: spec ms-vbal 5.4.5.1 Print # can never be used on a Random file
' XFAIL: wrong pending
' EXPECT: err=True
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Open "/t/m.dat" For Random As #1 Len = 16
    On Error Resume Next
    Print #1, "x"
    Debug.Print "err=" & CStr(Err.Number <> 0)
End Sub
