' INV: vbal.5.4.2.10, stmt.select-case
' SOURCE: spec ms-vbal 5.4.2.10 start To end is (sel >= start) And (sel <= end), so a reversed range never matches
' EXPECT: reversed-else
' EXPECT: bounds-inclusive
Public Sub Main()
    Select Case 5
        Case 9 To 1
            Debug.Print "reversed-match"
        Case Else
            Debug.Print "reversed-else"
    End Select
    Select Case 9
        Case 1 To 9
            Debug.Print "bounds-inclusive"
        Case Else
            Debug.Print "no"
    End Select
End Sub
