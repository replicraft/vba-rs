' INV: vbal.5.6.9.8.1, op.not
' SOURCE: spec ms-vbal 5.6.9.8.1 Not operator bitwise negation and its value types
' EXPECT: Byte 250
' EXPECT: Boolean False
' EXPECT: Boolean True
' EXPECT: Integer -6
' EXPECT: Long -100001
' EXPECT: LongLong -4294967297
Public Sub Main()
    Dim ll As LongLong
    Debug.Print TypeName(Not CByte(5)) & " " & (Not CByte(5))
    Debug.Print TypeName(Not True) & " " & (Not True)
    Debug.Print TypeName(Not False) & " " & (Not False)
    Debug.Print TypeName(Not CInt(5)) & " " & (Not CInt(5))
    Debug.Print TypeName(Not CLng(100000)) & " " & (Not CLng(100000))
    ll = 4294967296^
    Debug.Print TypeName(Not ll) & " " & (Not ll)
End Sub
