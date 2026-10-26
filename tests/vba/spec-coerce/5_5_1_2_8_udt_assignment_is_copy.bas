' INV: vbal.5.5.1.2.8, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.8 Let-coercion to and from a UDT, same UDT assignment makes a copy of the fields
' EXPECT: a=1,hi
' EXPECT: b=5,hi
Type P
    X As Long
    S As String
End Type
Public Sub Main()
    Dim a As P
    Dim b As P
    a.X = 1: a.S = "hi"
    b = a
    b.X = 5
    Debug.Print "a=" & CStr(a.X) & "," & a.S
    Debug.Print "b=" & CStr(b.X) & "," & b.S
End Sub
