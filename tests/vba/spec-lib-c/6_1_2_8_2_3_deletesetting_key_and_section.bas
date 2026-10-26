' INV: vbal.6.1.2.8.2.3, stmt.deletesetting
' SOURCE: spec ms-vbal 6.1.2.8.2.3 DeleteSetting removes one key, or a whole section when Key omitted
' EXPECT: before=1,2
' EXPECT: key=gone,2
' EXPECT: section=gone,gone
Public Sub Main()
    SaveSetting "VbarsConfApp4", "S", "K1", "1"
    SaveSetting "VbarsConfApp4", "S", "K2", "2"
    Debug.Print "before=" & GetSetting("VbarsConfApp4", "S", "K1", "gone") & "," & GetSetting("VbarsConfApp4", "S", "K2", "gone")
    DeleteSetting "VbarsConfApp4", "S", "K1"
    Debug.Print "key=" & GetSetting("VbarsConfApp4", "S", "K1", "gone") & "," & GetSetting("VbarsConfApp4", "S", "K2", "gone")
    DeleteSetting "VbarsConfApp4", "S"
    Debug.Print "section=" & GetSetting("VbarsConfApp4", "S", "K1", "gone") & "," & GetSetting("VbarsConfApp4", "S", "K2", "gone")
End Sub
