' INV: vbal.6.1.2.7.1.13, func.vartype
' SOURCE: spec ms-vbal 6.1.2.7.1.13 VarType of object reference, Missing, and arrays (8192 + element type)
' EXPECT: object=9
' EXPECT: missing=10
' EXPECT: intarray=8194
' EXPECT: strarray=8200
' EXPECT: varray=8204
' EXPECT: dblarray=8197
Private Function Probe(Optional p As Variant) As Long
    Probe = VarType(p)
End Function

Public Sub Main()
    Dim c As Collection
    Dim ia(1) As Integer
    Dim sa(1) As String
    Dim va(1) As Variant
    Dim da(1) As Double
    Set c = New Collection
    Debug.Print "object=" & CStr(VarType(c))
    Debug.Print "missing=" & CStr(Probe())
    Debug.Print "intarray=" & CStr(VarType(ia))
    Debug.Print "strarray=" & CStr(VarType(sa))
    Debug.Print "varray=" & CStr(VarType(va))
    Debug.Print "dblarray=" & CStr(VarType(da))
End Sub
