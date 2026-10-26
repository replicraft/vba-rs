' INV: vbal.5.2.3.3
' SOURCE: spec ms-vbal 5.2.3.3 As New UDT member makes the dependent variable auto-instantiating
' XFAIL: wrong VBA behaviour kept where MS-VBAL text differs: As New is rejected for arrays and Type members
' EXPECT: 9
Private Type Holder
    b As New Box
End Type
Public Sub Main()
    Dim h As Holder
    h.b.v = 9
    Debug.Print CStr(h.b.v)
End Sub
'#MODULE Box class
Public v As Long
