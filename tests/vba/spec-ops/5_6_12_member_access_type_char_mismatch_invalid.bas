' INV: vbal.5.6.12
' SOURCE: spec ms-vbal 5.6.12 member name type character must match the member's declared type
' XFAIL: wrong not implemented: type character check on member access
' ERROR: load
'#MODULE Box class
Public W As Long
'#MODULE Module1 standard
Public Sub Main()
    Dim b As New Box
    b.W = 1
    Debug.Print CStr(b.W$)
End Sub
