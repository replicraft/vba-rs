' INV: vbal.6.1.2.8.2.3, stmt.deletesetting
' SOURCE: spec ms-vbal 6.1.2.8.2.3 DeleteSetting on a non-existent Section or Key is a run-time error
' EXPECT: section=err
' EXPECT: key=err
Public Sub Main()
    On Error Resume Next
    DeleteSetting "VbarsConfApp5", "NoSuchSection"
    Debug.Print "section=" & IIf(Err.Number <> 0, "err", "noerr")
    Err.Clear
    SaveSetting "VbarsConfApp5", "Real", "K", "v"
    DeleteSetting "VbarsConfApp5", "Real", "NoSuchKey"
    Debug.Print "key=" & IIf(Err.Number <> 0, "err", "noerr")
    Err.Clear
    DeleteSetting "VbarsConfApp5"
End Sub
