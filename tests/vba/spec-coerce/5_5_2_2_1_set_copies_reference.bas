' INV: vbal.5.5.2.2.1, stmt.set
' SOURCE: spec ms-vbal 5.5.2.2.1 Set-coercion to and from a class, source and destination refer to the same object; Nothing is copied as Nothing; implemented interface and Object destinations are allowed
' EXPECT: shared=5
' EXPECT: object shared=6
' EXPECT: iface shared=7
' EXPECT: var shared=8
' EXPECT: nothing=True
'#MODULE IThing class
Public Sub Poke()
End Sub
'#MODULE Wrap class
Implements IThing
Public V As Long
Private Sub IThing_Poke()
    V = V + 100
End Sub
'#MODULE Module1 standard
Public Sub Main()
    Dim a As Wrap, b As Wrap
    Dim o As Object
    Dim i As IThing
    Dim v As Variant
    Set a = New Wrap
    Set b = a
    b.V = 5
    Debug.Print "shared=" & CStr(a.V)
    Set o = a
    o.V = 6
    Debug.Print "object shared=" & CStr(a.V)
    Set i = a
    a.V = 7
    Debug.Print "iface shared=" & CStr(a.V)
    Set v = a
    v.V = 8
    Debug.Print "var shared=" & CStr(a.V)
    Set b = Nothing
    Debug.Print "nothing=" & CStr(b Is Nothing)
End Sub
