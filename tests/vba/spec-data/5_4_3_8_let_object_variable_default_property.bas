' INV: vbal.5.4.3.8, stmt.let
' SOURCE: spec ms-vbal 5.4.3.8 Let on class-typed variable assigns the default property
' EXPECT: 42
'#MODULE C1 class
Private m As Long
Public Property Get V() As Long
Attribute V.VB_UserMemId = 0
    V = m
End Property
Public Property Let V(ByVal n As Long)
    m = n
End Property
'#MODULE Module1 standard
Public Sub Main()
    Dim c As New C1
    c = 42
    Debug.Print CStr(c.V)
End Sub
