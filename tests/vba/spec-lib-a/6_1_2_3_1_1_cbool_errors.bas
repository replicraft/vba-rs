' INV: vbal.6.1.2.3.1.1, func.cbool
' SOURCE: spec ms-vbal 6.1.2.3.1.1 CBool error cases
' EXPECT: hashLower=13
' EXPECT: text=13
' EXPECT: emptyStr=13
' EXPECT: null=94
Public Sub Main()
    Dim v As Variant
    On Error Resume Next
    Err.Clear: v = CBool("#true#"): Debug.Print "hashLower=" & Err.Number
    Err.Clear: v = CBool("abc"): Debug.Print "text=" & Err.Number
    Err.Clear: v = CBool(""): Debug.Print "emptyStr=" & Err.Number
    Err.Clear: v = CBool(Null): Debug.Print "null=" & Err.Number
End Sub
