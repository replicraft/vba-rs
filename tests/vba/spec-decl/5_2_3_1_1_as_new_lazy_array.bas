' INV: vbal.5.2.3.1.1
' SOURCE: spec ms-vbal 5.2.3.1.1 As New array: each dependent variable is automatic instantiation
' EXPECT: 5 0
Public Sub Main()
    Dim a(1) As New Box
    a(0).v = 5
    Debug.Print CStr(a(0).v) & " " & CStr(a(1).v)
End Sub
'#MODULE Box class
Public v As Long
