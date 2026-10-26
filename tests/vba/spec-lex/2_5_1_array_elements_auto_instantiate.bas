' INV: vbal.2.5.1, decl.dim-as-new
' SOURCE: spec ms-vbal 2.5.1 automatic object instantiation, elements of an array declared with New are automatic instantiation variables
' XFAIL: wrong pending
' EXPECT: created
' EXPECT: 7
' EXPECT: created
' EXPECT: 0
'#MODULE Box class
Public V As Long
Private Sub Class_Initialize()
    Debug.Print "created"
End Sub
'#MODULE Module1 standard
Public Sub Main()
    Dim a(1 To 2) As New Box
    a(1).V = 7
    Debug.Print CStr(a(1).V)
    Debug.Print CStr(a(2).V)
End Sub
