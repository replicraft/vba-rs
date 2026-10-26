' INV: vbal.5.4.5.2, stmt.close
' SOURCE: spec ms-vbal 5.4.5.2 Close of a number that is not open takes no action
' EXPECT: err=0
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    On Error Resume Next
    Close #5
    Close #5, #6
    Debug.Print "err=" & CStr(Err.Number)
End Sub
