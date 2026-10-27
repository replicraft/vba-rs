' INV: vbal.5.2.3.1.1, decl.dim-as-new
' SOURCE: spec ms-vbal 5.2.3.1.1 each element of an array declared As New is an automatic instantiation variable: dynamic arrays, ByRef element arguments and elements set to Nothing
' EXPECT: init
' EXPECT: init
' EXPECT: 4 0
' EXPECT: init
' EXPECT: False
' EXPECT: init
' EXPECT: 0
' EXPECT: init
' EXPECT: init
' EXPECT: 2 0
Dim g(1 To 2) As New Box
Sub Touch(ByRef b As Box)
    b.V = b.V + 1
End Sub
Public Sub Main()
    Dim a() As New Box
    ReDim a(2)
    a(1).V = 4
    Debug.Print CStr(a(1).V) & " " & CStr(a(2).V)
    Dim s(1) As New Box
    Debug.Print CStr(s(0) Is Nothing)
    Set s(0) = Nothing
    Debug.Print CStr(s(0).V)
    Touch g(2)
    Touch g(2)
    Debug.Print CStr(g(2).V) & " " & CStr(g(1).V)
End Sub
'#MODULE Box class
Public V As Long
Private Sub Class_Initialize()
    Debug.Print "init"
End Sub
