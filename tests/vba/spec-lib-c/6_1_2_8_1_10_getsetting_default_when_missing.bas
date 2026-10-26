' INV: vbal.6.1.2.8.1.10, func.getsetting
' SOURCE: spec ms-vbal 6.1.2.8.1.10 GetSetting returns Default when an item does not exist, zero-length string when Default omitted
' EXPECT: dflt=fallback
' EXPECT: omitted=0
' EXPECT: key=fallback2
' EXPECT: stored=val
Public Sub Main()
    Debug.Print "dflt=" & GetSetting("VbarsConfApp3", "NoSec", "NoKey", "fallback")
    Debug.Print "omitted=" & Len(GetSetting("VbarsConfApp3", "NoSec", "NoKey"))
    SaveSetting "VbarsConfApp3", "Sec", "Present", "val"
    Debug.Print "key=" & GetSetting("VbarsConfApp3", "Sec", "Absent", "fallback2")
    Debug.Print "stored=" & GetSetting("VbarsConfApp3", "Sec", "Present", "fallback3")
    DeleteSetting "VbarsConfApp3"
End Sub
