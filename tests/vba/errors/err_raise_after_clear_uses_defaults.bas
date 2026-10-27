' INV: vbal.6.1.3.2.1.2
' SOURCE: derived Err.Raise takes uncleared Err values only while Err is not cleared (MS-VBAL 6.1.3.2.1.2); Error with no argument describes the held error
' EXPECT: MySrc|MyDesc
' EXPECT: [] []
' EXPECT: 5|Invalid procedure call or argument
Public Sub Main()
    On Error Resume Next
    Err.Raise 5, "MySrc", "MyDesc"
    Debug.Print Err.Source & "|" & Err.Description
    Err.Clear
    Debug.Print "[" & Error$ & "] [" & Err.Description & "]"
    Err.Raise 5
    Debug.Print CStr(Err.Number) & "|" & Err.Description
End Sub
