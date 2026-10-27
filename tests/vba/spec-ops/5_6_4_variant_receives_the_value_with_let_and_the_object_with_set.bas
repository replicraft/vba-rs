' INV: vbal.5.6.4, expr.default-member
' SOURCE: derived 5.6.4 the binding context of the right side follows the statement: Let binds a value (the default member), Set binds the object
' METHOD: by hand
' EXPECT: Integer
' EXPECT: Box
'#MODULE Box class
Private mV As Integer
Public Property Get V() As Integer
Attribute V.VB_UserMemId = 0
    V = mV
End Property
Public Property Let V(ByVal x As Integer)
    mV = x
End Property
'#MODULE Module1 standard
Public Sub Main()
    Dim b As New Box
    b.V = 7
    Dim v As Variant
    v = b
    Debug.Print TypeName(v)
    Set v = b
    Debug.Print TypeName(v)
End Sub
