' INV: vbal.2.3
' SOURCE: spec ms-vbal 2.3 variables, a UDT variable starts with every element at the default of its declared type
' EXPECT: 0 [] Empty False
' EXPECT: 0 0
Private Type T
    N As Long
    S As String
    V As Variant
    B As Boolean
    Inner As U
End Type
Private Type U
    X As Double
    Y As Integer
End Type
Public Sub Main()
    Dim r As T
    Debug.Print r.N & " [" & r.S & "] " & TypeName(r.V) & " " & CStr(r.B)
    Debug.Print r.Inner.X & " " & r.Inner.Y
End Sub
