' INV: vbal.5.2.4.2
' SOURCE: spec ms-vbal 5.2.4.2 public variable of interface implemented by property get and let
' EXPECT: 17
Public Sub Main()
    Dim h As IHolder
    Dim i As New Impl
    Set h = i
    h.Size = 17
    Debug.Print CStr(h.Size)
End Sub
'#MODULE IHolder class
Public Size As Long
'#MODULE Impl class
Implements IHolder
Private stored As Long
Private Property Get IHolder_Size() As Long
    IHolder_Size = stored
End Property
Private Property Let IHolder_Size(ByVal v As Long)
    stored = v
End Property
