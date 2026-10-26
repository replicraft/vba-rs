' INV: vbal.5.6.9.8, op.and, op.or, op.not
' SOURCE: spec ms-vbal 5.6.9.8 logical operators with Empty operands use Integer
' EXPECT: Integer 0
' EXPECT: Integer 5
' EXPECT: Integer -1
' EXPECT: Integer 0
Public Sub Main()
    Dim e As Variant, i As Integer
    i = 5
    Debug.Print TypeName(e And e) & " " & (e And e)
    Debug.Print TypeName(e Or i) & " " & (e Or i)
    Debug.Print TypeName(Not e) & " " & (Not e)
    Debug.Print TypeName(i And e) & " " & (i And e)
End Sub
