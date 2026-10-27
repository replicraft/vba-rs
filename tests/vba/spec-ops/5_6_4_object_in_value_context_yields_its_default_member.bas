' INV: vbal.5.6.4, expr.default-member
' SOURCE: derived 5.6.4 an expression is bound as a value where a value is required: an object there is replaced by its default member, while Set and TypeName keep the object
' METHOD: by hand: the box holds 7
' EXPECT:  7 
' EXPECT:  8 
' EXPECT: Box
' EXPECT: Box
' EXPECT:  7 
' EXPECT: Integer
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
    Dim n As Long
    n = b
    Debug.Print n
    Debug.Print b + 1
    Dim o As Object
    Set o = b
    Debug.Print TypeName(o)
    Debug.Print TypeName(b)
    Debug.Print o.V
    Debug.Print TypeName(b.V)
End Sub
