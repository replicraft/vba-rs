' INV: vbal.5.3.1.5
' SOURCE: spec ms-vbal 5.3.1.5 declared type of untyped, suffixed and Def-typed parameters
' EXPECT: Variant-ok
' EXPECT: Integer
' EXPECT: String
' EXPECT: Long 4
DefLng X
Sub A(p)
    p = "text"
    Debug.Print "Variant-ok"
End Sub
Sub B(ByVal n%, ByVal s$)
    Debug.Print TypeName(n)
    Debug.Print TypeName(s)
End Sub
Sub C(ByVal xa)
    Debug.Print TypeName(xa) & " " & CStr(xa)
End Sub
Public Sub Main()
    Dim v As Variant
    A v
    B 1, "z"
    C 3.7
End Sub
