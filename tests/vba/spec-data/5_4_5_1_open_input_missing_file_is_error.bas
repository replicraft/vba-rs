' INV: vbal.5.4.5.1, stmt.open
' SOURCE: spec ms-vbal 5.4.5.1 Input on a missing file generates an error and creates nothing
' EXPECT: err=True
' EXPECT: exists=[]
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    On Error Resume Next
    Open "/t/none.txt" For Input As #1
    Debug.Print "err=" & CStr(Err.Number <> 0)
    On Error GoTo 0
    Debug.Print "exists=[" & Dir("/t/none.txt") & "]"
End Sub
