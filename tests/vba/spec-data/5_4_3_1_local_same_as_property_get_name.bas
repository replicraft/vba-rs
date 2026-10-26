' INV: vbal.5.4.3.1, stmt.dim
' SOURCE: spec ms-vbal 5.4.3.1 variable must not have the containing property get name
' XFAIL: wrong VBA behaviour kept where MS-VBAL text differs: a local may share the name of the enclosing Function or Property Get
' ERROR: load
'#MODULE C1 class
Public Property Get P() As Integer
    Dim P As Integer
    P = 1
End Property
'#MODULE Module1 standard
Public Sub Main()
    Dim c As C1
    Set c = New C1
    Debug.Print c.P
End Sub
