' INV: vbal.5.3.1.7
' SOURCE: spec ms-vbal 5.3.1.7 Get, Let and Set declarations may share one name; indexed parameters
' EXPECT: let=5
' EXPECT: get(2)=50
' EXPECT: set=Cls
'#MODULE Cls class
Private store(0 To 3) As Long
Private obj As Object
Public Property Get Item(ByVal i As Long) As Long
    Item = store(i)
End Property
Public Property Let Item(ByVal i As Long, ByVal v As Long)
    store(i) = v
End Property
Public Property Get Ref() As Object
    Set Ref = obj
End Property
Public Property Set Ref(ByVal o As Object)
    Set obj = o
End Property
'#MODULE Module1 standard
Public Sub Main()
    Dim c As New Cls
    c.Item(2) = 50
    c.Item(1) = 5
    Debug.Print "let=" & CStr(c.Item(1))
    Debug.Print "get(2)=" & CStr(c.Item(2))
    Set c.Ref = c
    Debug.Print "set=" & TypeName(c.Ref)
End Sub
