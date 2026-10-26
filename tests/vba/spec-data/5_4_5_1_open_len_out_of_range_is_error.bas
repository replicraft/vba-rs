' INV: vbal.5.4.5.1, stmt.open
' SOURCE: spec ms-vbal 5.4.5.1 Len must be in 1 to 32767
' EXPECT: err=True
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    On Error Resume Next
    Open "/t/len.dat" For Random As #1 Len = 40000
    Debug.Print "err=" & CStr(Err.Number <> 0)
End Sub
