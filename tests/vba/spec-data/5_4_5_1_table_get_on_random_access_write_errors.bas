' INV: vbal.5.4.5.1, stmt.get
' SOURCE: spec ms-vbal 5.4.5.1 Get # needs read access on Random
' EXPECT: err=True
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim n As Long
    Open "/t/m.dat" For Random Access Write As #1 Len = 4
    On Error Resume Next
    Get #1, 1, n
    Debug.Print "err=" & CStr(Err.Number <> 0)
End Sub
