' INV: vbal.5.6.9.8, op.and, op.or
' SOURCE: spec ms-vbal 5.6.9.8 binary logical operator declared/effective types Byte Boolean Integer
' EXPECT: Byte 8
' EXPECT: Boolean True
' EXPECT: Integer 12
' EXPECT: Integer 1
' EXPECT: Integer 14
' EXPECT: Integer 8
' EXPECT: Long 8
Public Sub Main()
    Dim by As Byte, by2 As Byte, bo As Boolean, i As Integer, l As Long
    by = 12: by2 = 10: bo = True: i = 10: l = 12
    Debug.Print TypeName(by And by2) & " " & (by And by2)
    Debug.Print TypeName(bo And bo) & " " & (bo And bo)
    Debug.Print TypeName(by And bo) & " " & (by And bo)
    Debug.Print TypeName(bo And by) & " " & (bo And CByte(1))
    Debug.Print TypeName(by Or i) & " " & (by Or i)
    Debug.Print TypeName(bo And i) & " " & (bo And 8)
    Debug.Print TypeName(i And l) & " " & (i And l)
End Sub
