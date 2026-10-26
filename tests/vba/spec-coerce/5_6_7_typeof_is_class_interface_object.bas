' INV: vbal.5.6.7, expr.typeof
' SOURCE: spec ms-vbal 5.6.7 TypeOf...Is expressions, exact type, implemented interface and Object are True; other types False
' EXPECT: exact=True
' EXPECT: iface=True
' EXPECT: object=True
' EXPECT: other=False
' EXPECT: variant exact=True
' EXPECT: variant nonobject=False
' EXPECT: declared exact=True
' EXPECT: declared iface=True
'#MODULE IThing class
Public Sub Poke()
End Sub
'#MODULE Wrap class
Implements IThing
Private Sub IThing_Poke()
End Sub
'#MODULE Other class
Public Sub X()
End Sub
'#MODULE Module1 standard
Public Sub Main()
    Dim o As Object
    Dim v As Variant
    Dim w As Wrap
    Set o = New Wrap
    Debug.Print "exact=" & CStr(TypeOf o Is Wrap)
    Debug.Print "iface=" & CStr(TypeOf o Is IThing)
    Debug.Print "object=" & CStr(TypeOf o Is Object)
    Debug.Print "other=" & CStr(TypeOf o Is Other)
    Set v = New Wrap
    Debug.Print "variant exact=" & CStr(TypeOf v Is Wrap)
    v = 5
    Debug.Print "variant nonobject=" & CStr(TypeOf v Is Object)
    Set w = New Wrap
    Debug.Print "declared exact=" & CStr(TypeOf w Is Wrap)
    Debug.Print "declared iface=" & CStr(TypeOf w Is IThing)
End Sub
