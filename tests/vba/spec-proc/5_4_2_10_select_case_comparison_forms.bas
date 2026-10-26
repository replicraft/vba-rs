' INV: vbal.5.4.2.10, stmt.select-case
' SOURCE: spec ms-vbal 5.4.2.10 Is/comparison-operator clauses and start To end ranges
' EXPECT: 3:lt5
' EXPECT: 5:eq5
' EXPECT: 7:7to9
' EXPECT: 12:ne10
' EXPECT: 10:ge10
Function Classify(ByVal n As Long) As String
    Select Case n
        Case Is < 5
            Classify = "lt5"
        Case Is = 5
            Classify = "eq5"
        Case 7 To 9
            Classify = "7to9"
        Case 10
            Classify = "ge10"
        Case Is <> 11
            Classify = "ne10"
        Case Else
            Classify = "else"
    End Select
End Function
Public Sub Main()
    Debug.Print "3:" & Classify(3)
    Debug.Print "5:" & Classify(5)
    Debug.Print "7:" & Classify(7)
    Debug.Print "12:" & Classify(12)
    Debug.Print "10:" & Classify(10)
End Sub
