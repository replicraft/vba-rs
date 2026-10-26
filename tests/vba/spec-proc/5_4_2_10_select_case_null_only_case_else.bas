' INV: vbal.5.4.2.10, stmt.select-case
' SOURCE: spec ms-vbal 5.4.2.10 a Null select expression runs only the Case Else block
' EXPECT: else
Public Sub Main()
    Dim v As Variant
    v = Null
    Select Case v
        Case 0
            Debug.Print "zero"
        Case Is >= 0
            Debug.Print "ge"
        Case Is < 0
            Debug.Print "lt"
        Case Else
            Debug.Print "else"
    End Select
End Sub
