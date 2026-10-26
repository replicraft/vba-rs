' INV: vbal.2.3
' SOURCE: spec ms-vbal 2.3 variables, a fixed-size array starts with every element the default of the element type (Empty for Variant)
' EXPECT: 0 0 0
' EXPECT: False False
' EXPECT: [] []
' EXPECT: Empty Empty
' EXPECT: True
' EXPECT: 1 4
Public Sub Main()
    Dim a(1 To 3) As Long
    Dim b(0 To 1) As Boolean
    Dim s(0 To 1) As String
    Dim v(0 To 1) As Variant
    Dim o(0 To 0) As Object
    Dim m(1 To 2, 3 To 4) As Integer
    Debug.Print a(1) & " " & a(2) & " " & a(3)
    Debug.Print CStr(b(0)) & " " & CStr(b(1))
    Debug.Print "[" & s(0) & "] [" & s(1) & "]"
    Debug.Print TypeName(v(0)) & " " & TypeName(v(1))
    Debug.Print CStr(o(0) Is Nothing)
    Debug.Print LBound(m, 1) & " " & UBound(m, 2)
End Sub
