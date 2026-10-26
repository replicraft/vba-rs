' INV: vbal.5.5.2.1, stmt.set
' SOURCE: spec ms-vbal 5.5.2.1 Set-coercion static semantics, Set of an object into a Variant destination is valid and shares the object
' EXPECT: same=True
' EXPECT: type=Wrap
' EXPECT: nothing=True
'#MODULE Wrap class
Public V As Long
'#MODULE Module1 standard
Public Sub Main()
    Dim w As Wrap
    Dim v As Variant
    Set w = New Wrap
    Set v = w
    Debug.Print "same=" & CStr(v Is w)
    Debug.Print "type=" & TypeName(v)
    Set w = Nothing
    Set v = Nothing
    Debug.Print "nothing=" & CStr(v Is Nothing)
End Sub
