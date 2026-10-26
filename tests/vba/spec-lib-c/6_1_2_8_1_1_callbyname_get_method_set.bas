' INV: vbal.6.1.2.8.1.1, func.callbyname
' SOURCE: spec ms-vbal 6.1.2.8.1.1 CallByName vbGet vbMethod vbSet, last argument is the value for vbSet
' EXPECT: get=42
' EXPECT: method=7
' EXPECT: set=Hello
' EXPECT: consts=2 4 1 8
'#MODULE Box class
Public Size As Long
Private mRef As Object
Public Function Add(ByVal a As Long, ByVal b As Long) As Long
    Add = a + b
End Function
Public Property Get Ref() As Object
    Set Ref = mRef
End Property
Public Property Set Ref(ByVal v As Object)
    Set mRef = v
End Property
'#MODULE Main1 standard
Public Sub Main()
    Dim b As New Box, c As New Box
    b.Size = 42
    Debug.Print "get=" & CStr(CallByName(b, "Size", vbGet))
    Debug.Print "method=" & CStr(CallByName(b, "Add", vbMethod, 3, 4))
    c.Size = 5
    CallByName b, "Ref", vbSet, c
    Debug.Print "set=" & IIf(b.Ref Is c, "Hello", "Bad")
    Debug.Print "consts=" & vbGet & " " & vbLet & " " & vbMethod & " " & vbSet
End Sub
