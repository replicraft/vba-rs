' INV: vbal.5.6.1
' SOURCE: derived 5.6.1 a property of an object classifies as a property expression: it is read where a value is required and assigned where a variable is required
' METHOD: by hand: 3 then 3 + 4
' EXPECT:  3 
' EXPECT:  7 
'#MODULE Box class
Private mV As Long
Public Property Get V() As Long
    V = mV
End Property
Public Property Let V(ByVal x As Long)
    mV = x
End Property
'#MODULE Module1 standard
Public Sub Main()
    Dim b As New Box
    b.V = 3
    Debug.Print b.V
    b.V = b.V + 4
    Debug.Print b.V
End Sub
