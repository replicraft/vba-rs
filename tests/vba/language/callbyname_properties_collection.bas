' INV: func.callbyname
' SOURCE: unverified the call types follow the CallByName page; errors 438 (private member) and 5 (unknown call type) are assumptions
' EXPECT: 7
' EXPECT: x
' EXPECT: 2
' EXPECT: 438
' EXPECT: 5
'#MODULE Holder class
Private mV As Long
Private mObj As Object
Public Property Get V() As Long
    V = mV
End Property
Public Property Let V(ByVal n As Long)
    mV = n
End Property
Public Property Get Obj() As Object
    Set Obj = mObj
End Property
Public Property Set Obj(ByVal o As Object)
    Set mObj = o
End Property
Private Sub Hidden()
End Sub
'#MODULE Module1 standard
Public Sub Main()
    Dim h As New Holder, c As New Collection, o As Object
    CallByName h, "V", VbLet, 7
    Debug.Print CStr(CallByName(h, "V", VbGet))
    c.Add "x"
    CallByName h, "Obj", VbSet, c
    Set o = CallByName(h, "Obj", VbGet)
    Debug.Print CallByName(o, "Item", VbMethod, 1)
    CallByName c, "Add", VbMethod, "y"
    Debug.Print CStr(CallByName(c, "Count", VbGet))
    On Error Resume Next
    CallByName h, "Hidden", VbMethod
    Debug.Print Err.Number & ""
    Err.Clear
    CallByName h, "V", 3
    Debug.Print Err.Number & ""
End Sub
