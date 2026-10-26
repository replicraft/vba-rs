' INV: vbal.5.6.10
' SOURCE: spec ms-vbal 5.6.10 type character not matching the declared type makes the name invalid
' XFAIL: wrong pending
' ERROR: load
Public Sub Main()
    Dim n As Long
    n = 1
    Debug.Print CStr(n$)
End Sub
