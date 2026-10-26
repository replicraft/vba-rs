' INV: vbal.5.4.4.1, stmt.on-error
' SOURCE: spec ms-vbal 5.4.4.1 On Error resets the Err object
' XFAIL: wrong pending
' EXPECT: in=11
' EXPECT: after=0
Public Sub Main()
    Dim x As Long
    On Error Resume Next
    x = 1 / 0
    Debug.Print "in=" & CStr(Err.Number)
    On Error Resume Next
    Debug.Print "after=" & CStr(Err.Number)
End Sub
